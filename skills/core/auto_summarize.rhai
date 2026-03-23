// @name: 자동 요약
// @desc: 파일을 읽고 내용을 자동으로 요약합니다
//
// [v0.2.0] Rhai 동적 스킬 예제
// workspace/data/ 내 파일을 읽고 요약문을 생성

let files = file_list("data");
print("데이터 파일 수: " + files.len());

for f in files {
    let content = file_read("data/" + f);
    print("파일: " + f + " (" + content.len() + "자)");
}

print("분석 완료!");
