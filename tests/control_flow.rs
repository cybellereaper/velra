use velra::{compile, compile_self_hosted, run_self_hosted, Value};

fn eval(source: &str) -> Value {
    run_self_hosted(source).unwrap()
}

#[test]
fn while_break_continue_and_compound_assignment_work() {
    let source = r#"
var index = 0
var total = 0
while index < 10 {
    index += 1
    if index == 3 { continue }
    if index == 8 { break }
    total += index
}
total
"#;
    assert_eq!(eval(source).to_string(), "25");
}

#[test]
fn membership_slices_and_safe_get_are_composable() {
    assert_eq!(eval("\"b\" in [\"a\", \"b\"]").to_string(), "true");
    assert_eq!(eval("\"ell\" in \"hello\"").to_string(), "true");
    assert_eq!(eval("[1, 2, 3, 4][1..3]").to_string(), "[2, 3]");
    assert_eq!(eval("\"hello\"[1..4]").to_string(), "ell");
    assert_eq!(eval("[10].get(4)").to_string(), "null");
    assert_eq!(eval("\"ab\".get(1)").to_string(), "b");
}

#[test]
fn cursor_api_handles_lookahead_and_consumption() {
    let source = r#"
input = "abc".cursor()
head = input.current()
lookahead = input.peek()
prefix = input.peek_string(2)
input.advance()
consumed = input.take()
[head, lookahead, prefix, consumed, input.position(), input.done()]
"#;
    assert_eq!(eval(source).to_string(), "[a, b, ab, b, 2, false]");
}

#[test]
fn self_hosted_compiler_matches_stage_zero_for_control_flow() {
    let source = r#"
var index = 0
var output = ""
while index < 4 {
    index += 1
    if index == 2 { continue }
    output += "${index}"
}
inside = "23" in output
piece = output[0..2]
safe = output.get(99) ?: "missing"
input = output.cursor()
first = input.take()
[inside, piece, safe, first]
"#;
    assert_eq!(
        compile(source).unwrap(),
        compile_self_hosted(source).unwrap()
    );
}
