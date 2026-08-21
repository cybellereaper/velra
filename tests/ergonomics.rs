use velra::{compile, compile_self_hosted, run, run_self_hosted};

fn assert_both(source: &str, expected: &str) {
    assert_eq!(run(source).unwrap().to_string(), expected);
    assert_eq!(run_self_hosted(source).unwrap().to_string(), expected);
}

#[test]
fn constructor_and_list_destructuring_are_first_class() {
    assert_both(
        r#"
User(name: String, age: Int) {}
user = User("Ada", 20)
User(name, age) = user
[one, two, three] = 1..=3
require name == "Ada"
require age == 20
require [one, two, three] == [1, 2, 3]
name
"#,
        "Ada",
    );
}

#[test]
fn when_supports_structural_patterns_bindings_wildcards_and_guards() {
    assert_both(
        r#"
User(name: String, age: Int) {}
describe(user) {
    when user {
        User(name, age) if age >= 18 => name
        User(name, _) => name + " (minor)"
        _ => "unknown"
    }
}
describe(User("Ada", 20))
"#,
        "Ada",
    );
}

#[test]
fn method_style_calls_reuse_global_functions() {
    assert_both(
        r#"
value = "  velra  ".trim().upper()
require value.contains("VEL")
require !value.is_empty()
require [1, 2, 3].first() == 1
require [1, 2, 3].last() == 3
value
"#,
        "VELRA",
    );
}

#[test]
fn ranges_and_pipelines_lower_to_existing_calls() {
    assert_both(
        r#"
double(value) => value * 2
exclusive = 1..4
inclusive = 1..=4
require exclusive == [1, 2, 3]
require inclusive == [1, 2, 3, 4]
5 |> double
"#,
        "10",
    );
}

#[test]
fn self_hosted_compiler_matches_stage_zero_for_new_syntax() {
    let source = r#"
User(name: String, age: Int) {}
user = User("Ada", 20)
User(name, age) = user
[one, two] = 1..=2
require " velra ".trim().upper() == "VELRA"
result = when user {
    User(person, years) if years >= 18 => person
    _ => "minor"
}
result |> println
"#;
    assert_eq!(
        compile(source).unwrap(),
        compile_self_hosted(source).unwrap()
    );
}
