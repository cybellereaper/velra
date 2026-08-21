use velra::{
    ast::{Expr, Program, Stmt},
    compile, load_artifact, Interpreter, Value,
};

fn compiler_program() -> (String, Program) {
    let source = std::fs::read_to_string("compiler/main.vel").unwrap();
    let artifact = compile(&source).unwrap();
    let program = load_artifact(&artifact).unwrap();
    (source, program)
}

fn invoke(mut program: Program, name: &str, argument: &str) -> Value {
    program.statements.push(Stmt::Expr(Expr::Call {
        callee: Box::new(Expr::Ident(name.into())),
        args: vec![Expr::String(argument.into())],
    }));
    Interpreter::new().eval_program(&program).unwrap()
}

fn invoke_compiler(program: Program, source: &str) -> String {
    match invoke(program, "compile", source) {
        Value::String(artifact) => artifact,
        value => panic!("compiler returned {}, expected String", value.type_name()),
    }
}

#[test]
fn compiled_artifacts_execute() {
    let artifact = compile("answer = 40 + 2\nanswer").unwrap();
    let program = load_artifact(&artifact).unwrap();
    assert_eq!(
        Interpreter::new()
            .eval_program(&program)
            .unwrap()
            .to_string(),
        "42"
    );
}

#[test]
fn compiled_compiler_helpers_run() {
    let (_, compiler) = compiler_program();
    assert!(matches!(
        invoke(compiler.clone(), "is_digit", "1"),
        Value::Bool(true)
    ));
    assert!(matches!(
        invoke(compiler.clone(), "quote", "x"),
        Value::String(value) if value == "\"x\""
    ));
    assert!(matches!(
        invoke(compiler.clone(), "lex", ""),
        Value::List(_)
    ));
    assert!(matches!(invoke(compiler, "lex", "x"), Value::List(_)));
}

#[test]
fn compiled_compiler_lexes_its_source() {
    let (source, compiler) = compiler_program();
    assert!(matches!(invoke(compiler, "lex", &source), Value::List(_)));
}

#[test]
fn compiled_compiler_matches_stage_zero_on_minimal_source() {
    let (_, compiler) = compiler_program();
    let source = "answer = 40 + 2\nanswer";
    let stage_zero = compile(source).unwrap();
    let stage_one = invoke_compiler(compiler, source);
    assert_eq!(stage_one, stage_zero);
}

#[test]
fn compiler_reproduces_its_stage_zero_artifact() {
    let (source, compiler) = compiler_program();
    let stage_one = compile(&source).unwrap();
    let stage_two = invoke_compiler(compiler, &source);
    assert_eq!(stage_two, stage_one);
}
