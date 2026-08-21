use velra::{compile, compile_self_hosted, run, run_self_hosted};

fn assert_both(source: &str, expected: &str) {
    assert_eq!(run(source).unwrap().to_string(), expected);
    assert_eq!(run_self_hosted(source).unwrap().to_string(), expected);
}

#[test]
fn result_and_option_variants_support_propagation() {
    assert_both(
        r#"
bump(result) {
    value = result?
    Ok(value + 1)
}
result = bump(Ok(41))
require result.is_ok()
require result.unwrap() == 42
fallback = Err("bad").unwrap_or(9)
require fallback == 9
when bump(Err("bad")) {
    Err(message) => message
    else => "wrong"
}
"#,
        "bad",
    );

    assert_both(
        r#"
double(option) {
    value = option?
    Some(value * 2)
}
require double(Some(3)).is_some()
require double(Some(3)).unwrap() == 6
require None().is_none()
when double(None()) {
    None() => "none"
    else => "wrong"
}
"#,
        "none",
    );
}

#[test]
fn strings_interpolate_expressions_and_escape_dollars() {
    assert_both(
        r#"
name = "Ada"
count = 2
"Hello ${name}, count=${count + 1}, $$5"
"#,
        "Hello Ada, count=3, $5",
    );
}

#[test]
fn expression_lambdas_are_first_class() {
    assert_both(
        r#"
apply(value, fn) => fn(value)
combine(a, b, fn) => fn(a, b)
require apply(21, |x| x * 2) == 42
combine(20, 22, |left, right| left + right)
"#,
        "42",
    );
}

#[test]
fn for_loops_accept_structural_patterns() {
    assert_both(
        r#"
Point(x: Int, y: Int) {}
var total = 0
for Point(x, y) in [Point(1, 2), Point(3, 4)] {
    total = total + x + y
}
total
"#,
        "10",
    );
}

#[test]
fn self_hosted_compiler_matches_stage_zero_for_v2_syntax() {
    let source = r#"
Point(x: Int, y: Int) {}
apply(value, fn) => fn(value)
convert(result) {
    value = result?
    Ok("value=${value}")
}
var total = 0
for Point(x, y) in [Point(1, 2)] {
    total = total + x + y
}
apply(convert(Ok(total)).unwrap(), |text| text.upper())
"#;
    assert_eq!(
        compile(source).unwrap(),
        compile_self_hosted(source).unwrap()
    );
}
