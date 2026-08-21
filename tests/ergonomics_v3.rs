use velra::{compile, compile_self_hosted, run_self_hosted, Value};

fn eval(source: &str) -> Value {
    run_self_hosted(source).unwrap()
}

#[test]
fn enums_support_payload_and_singleton_variants() {
    let source = r#"
enum Token {
    Ident(text: String)
    Number(value: Int)
    Eof
}

describe(token: Token) => when token {
        Ident(text) => "id:${text}"
        Number(value) => "number:${value}"
        Eof => "eof"
    }

[describe(Ident("name")), describe(Number(42)), describe(Eof), type(Eof)]
"#;
    assert_eq!(eval(source).to_string(), "[id:name, number:42, eof, Token]");
}

#[test]
fn maps_sets_and_rest_patterns_are_first_class() {
    let source = r#"
keywords = {"if": "kw-if", "while": "kw-while"}
symbols = #{"==", "!=", "=="}
result = when [1, 2, 3, 4] {
    [first_value, second_value, ...rest] => [first_value, second_value, rest]
    else => []
}
[keywords.get("if"), keywords["while"], "==" in symbols, len(symbols), result]
"#;
    assert_eq!(
        eval(source).to_string(),
        "[kw-if, kw-while, true, 2, [1, 2, [3, 4]]]"
    );
}

#[test]
fn richer_cursor_and_unicode_character_methods_work() {
    let source = r#"
input = "123...β".cursor()
digits = input.take_while(|ch| ch.is_digit())
symbol = input.match_longest(#{"..", "..."})
greek = input.take()
[digits, symbol, greek.is_letter(), greek.is_lower(), input.done()]
"#;
    assert_eq!(eval(source).to_string(), "[123, ..., true, true, true]");
}

#[test]
fn bound_when_subjects_avoid_repeated_lookups() {
    let source = r#"
when ch = "Δ" {
    _ if ch.is_upper() => ch.lower()
    else => "no"
}
"#;
    assert_eq!(eval(source).to_string(), "δ");
}

#[test]
fn self_hosted_compiler_matches_stage_zero_for_v3_syntax() {
    let source = r#"
enum ResultToken { Word(text), Eof }
lookup = {"if": Word("if")}
symbols = #{"..", "..."}
input = "...abc".cursor()
matched = input.match_longest(symbols)
rest = input.take_while(|ch| ch.is_letter())
value = when ch = rest.first() {
    _ if ch.is_letter() => lookup.get("if")
    else => Eof
}
when [matched, rest, value] {
    [head, ...tail] => [head, tail]
    else => []
}
"#;
    assert_eq!(
        compile(source).unwrap(),
        compile_self_hosted(source).unwrap()
    );
}
