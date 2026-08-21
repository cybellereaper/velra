use velra::{check, run};

#[test]
fn fibonacci_program() {
    let source = r#"
fib(n) {
    if n < 2 {
        return n
    }
    fib(n - 1) + fib(n - 2)
}
fib(10)
"#;
    assert_eq!(run(source).unwrap().to_string(), "55");
}

#[test]
fn for_loop_and_mutation() {
    let source = r#"
var total = 0
for n in 5 {
    total = total + n
}
total
"#;
    assert_eq!(run(source).unwrap().to_string(), "10");
}

#[test]
fn null_safe_access_and_elvis() {
    let source = "user = null\nuser?.name ?: \"anonymous\"";
    assert_eq!(run(source).unwrap().to_string(), "anonymous");
}

#[test]
fn rejects_reserved_features_cleanly() {
    let error = check("async work() => 1").unwrap_err().to_string();
    assert!(error.contains("reserved"));
}

#[test]
fn explicit_types_are_runtime_contracts() {
    assert_eq!(
        run("var value: Number = 1\nvalue").unwrap().to_string(),
        "1"
    );
    let error = run("var value: Int = \"wrong\"").unwrap_err().to_string();
    assert!(error.contains("expects Int"));
}
