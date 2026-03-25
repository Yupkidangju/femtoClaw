// femtoClaw — Tool Protocol 모듈
// [v0.6.0] OpenAI Function Calling 프로토콜 구현
//
// 설계 원칙:
//   - BUILTIN_TOOLS를 OpenAI tools 파라미터 형식(JSON Schema)으로 변환
//   - LLM 응답의 tool_calls를 파싱하여 executor에 전달
//   - 실행 결과를 tool role 메시지로 변환하여 LLM에 재주입
//   - 최대 tool_call 연쇄 횟수 제한 (무한 루프 방지)

use super::agent::{FunctionDefinition, ToolCall, ToolDefinition};
use crate::tools::executor::{ToolExecutor, ToolResult};
use crate::tools::registry::BUILTIN_TOOLS;

/// [v0.6.0] BUILTIN_TOOLS를 OpenAI Function Calling tools 파라미터로 변환
pub fn build_tool_definitions() -> Vec<ToolDefinition> {
    BUILTIN_TOOLS
        .iter()
        .map(|tool| {
            // 파라미터를 JSON Schema properties로 변환
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for param in tool.params {
                let mut prop = serde_json::Map::new();
                prop.insert("type".into(), serde_json::Value::String("string".into()));
                prop.insert(
                    "description".into(),
                    serde_json::Value::String(format!(
                        "{} (e.g., {})",
                        param.description, param.example
                    )),
                );
                properties.insert(param.name.to_string(), serde_json::Value::Object(prop));
                if param.required {
                    required.push(serde_json::Value::String(param.name.to_string()));
                }
            }

            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: tool.id.to_string(),
                    description: tool.description.to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": serde_json::Value::Object(properties),
                        "required": required,
                    }),
                },
            }
        })
        .collect()
}

/// [v0.6.0] tool_call의 arguments JSON을 파싱하여 (key, value) 쌍으로 변환
pub fn parse_tool_arguments(arguments_json: &str) -> Vec<(String, String)> {
    match serde_json::from_str::<serde_json::Value>(arguments_json) {
        Ok(serde_json::Value::Object(map)) => map
            .into_iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                (k, val)
            })
            .collect(),
        _ => Vec::new(), // 파싱 실패 시 빈 배열
    }
}

/// [v0.6.0] tool_call 하나를 실행하고 결과를 반환한다.
pub fn execute_tool_call(executor: &mut ToolExecutor, tool_call: &ToolCall) -> ToolResult {
    let args = parse_tool_arguments(&tool_call.function.arguments);
    let arg_refs: Vec<(&str, &str)> = args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    executor.execute(&tool_call.function.name, &arg_refs)
}

/// [v0.6.0] tool_call 실행 결과를 문자열로 포맷팅
pub fn format_tool_result(result: &ToolResult) -> String {
    if result.success {
        result.output.clone().unwrap_or_else(|| "OK".to_string())
    } else if let Some(ref err) = result.error {
        format!("Error: {}", err.user_message())
    } else {
        "Unknown error".to_string()
    }
}

/// [v0.6.0] 최대 연쇄 tool_call 횟수
pub const MAX_TOOL_ROUNDS: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tool_definitions() {
        let defs = build_tool_definitions();
        assert_eq!(defs.len(), 6); // file_read, file_write, file_list, sleep, print, run_skill

        // file_read 확인
        let file_read = defs
            .iter()
            .find(|d| d.function.name == "file_read")
            .unwrap();
        assert_eq!(file_read.tool_type, "function");
        assert!(file_read.function.description.contains("file"));

        // 파라미터 JSON Schema 확인
        let params = &file_read.function.parameters;
        assert!(params["properties"]["path"].is_object());
        assert!(params["required"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_parse_tool_arguments() {
        let args = parse_tool_arguments(r#"{"path": "data/report.txt"}"#);
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].0, "path");
        assert_eq!(args[0].1, "data/report.txt");
    }

    #[test]
    fn test_parse_tool_arguments_multiple() {
        let args = parse_tool_arguments(r#"{"path": "output.txt", "content": "hello world"}"#);
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_parse_tool_arguments_invalid() {
        let args = parse_tool_arguments("not json");
        assert!(args.is_empty());
    }

    #[test]
    fn test_execute_tool_call_file_read() {
        let ws = std::env::temp_dir().join("femtoclaw_tp_test");
        std::fs::create_dir_all(&ws).ok();
        std::fs::write(ws.join("test.txt"), "hello").ok();

        let mut executor = ToolExecutor::new(ws.clone());

        let tc = ToolCall {
            id: "call_1".into(),
            call_type: "function".into(),
            function: super::super::agent::FunctionCall {
                name: "file_read".into(),
                arguments: r#"{"path": "test.txt"}"#.into(),
            },
        };

        let result = execute_tool_call(&mut executor, &tc);
        assert!(result.success);
        assert_eq!(result.output.unwrap(), "hello");

        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn test_format_tool_result_success() {
        let result = ToolResult {
            tool_id: "file_read".into(),
            success: true,
            output: Some("file contents".into()),
            error: None,
            security_event: false,
        };
        assert_eq!(format_tool_result(&result), "file contents");
    }
}
