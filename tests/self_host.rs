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

fn call(name: &str, argument: &str) -> Program {
    Program {
        statements: vec![Stmt::Expr(Expr::Call {
            callee: Box::new(Expr::Ident(name.into())),
            args: vec![Expr::String(argument.into())],
        })],
    }
}

fn invoke(mut program: Program, name: &str, argument: &str) -> Value {
    program.statements.extend(call(name, argument).statements);
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
        invoke(compiler.clone(), "is_upper", "T"),
        Value::Bool(true)
    ));
    assert!(matches!(
        invoke(compiler.clone(), "is_letter", "T"),
        Value::Bool(true)
    ));
    assert!(matches!(
        invoke(compiler.clone(), "is_ident_start", "T"),
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
    assert!(matches!(
        invoke(compiler.clone(), "lex", "x"),
        Value::List(_)
    ));
    assert!(matches!(invoke(compiler, "lex", "T"), Value::List(_)));
}

#[test]
fn compiled_compiler_lexer_accepts_symbols() {
    let (_, compiler) = compiler_program();
    let mut interpreter = Interpreter::new();
    interpreter.eval_program(&compiler).unwrap();

    for input in ["(", ",", ")", "{", "}", "Token(", "Token(kind", "Token(kind,", "Token(kind, text)", "Token(kind, text) {}"] {
        if let Err(error) = interpreter.eval_program(&call("lex", input)) {
            panic!("lexer failed on {input:?}: {error}");
        }
    }
}

#[test]
fn compiled_compiler_lexer_accepts_each_source_line() {
    let (source, compiler) = compiler_program();
    let mut interpreter = Interpreter::new();
    interpreter.eval_program(&compiler).unwrap();

    for (index, line) in source.lines().enumerate() {
        if let Err(error) = interpreter.eval_program(&call("lex", line)) {
            panic!("lexer failed on line {} ({line:?}): {error}", index + 1);
        }
    }
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
