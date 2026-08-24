use velra::{artifact, compile, lexer, run, run_artifact};

#[test]
fn unicode_string_indexing_and_slicing_use_character_offsets() {
    assert_eq!(run("get(\"aé🙂z\", 1)").unwrap().to_string(), "é");
    assert_eq!(run("get(\"aé🙂z\", -2)").unwrap().to_string(), "🙂");
    assert_eq!(run("slice(\"aé🙂z\", 1, 3)").unwrap().to_string(), "é🙂");
    assert_eq!(
        run("slice_inclusive(\"aé🙂z\", -3, -2)")
            .unwrap()
            .to_string(),
        "é🙂"
    );
}

#[test]
fn string_bounds_fail_cleanly() {
    assert_eq!(run("get(\"abc\", -999999)").unwrap().to_string(), "null");

    let error = run("\"abc\"[3]").unwrap_err().to_string();
    assert!(error.contains("out of bounds"));

    let error = run("slice(\"abc\", 0, 4)").unwrap_err().to_string();
    assert!(error.contains("out of bounds"));
}

#[test]
fn artifact_round_trip_preserves_unicode_and_escapes() {
    let source = "value = \"é🙂\\n\\\"ok\\\"\"\nvalue";
    let encoded = compile(source).unwrap();

    assert_eq!(run_artifact(&encoded).unwrap().to_string(), "é🙂\n\"ok\"");
}

#[test]
fn artifact_errors_report_utf8_byte_offsets() {
    const HEADER: &str = "VELRA-AST-1\n";
    let artifact = format!("{HEADER}(\"é\\x\")");
    let error = artifact::decode(&artifact).unwrap_err();

    assert_eq!(error.offset, HEADER.len() + 5);
}

#[test]
fn lexer_errors_cover_the_full_multibyte_character() {
    let error = lexer::lex("🙂").unwrap_err();

    assert_eq!(error.span.start, 0);
    assert_eq!(error.span.end, "🙂".len());
}
