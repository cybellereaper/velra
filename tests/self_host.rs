use velra::{
    ast::{Expr, Program, Stmt},
    compile, load_artifact, Interpreter, Value,
};

fn invoke_compiler(mut program: Program, source: &str) -> String {
    program.statements.push(Stmt::Expr(Expr::Call {
        callee: Box::new(Expr::Ident("compile".into())),
        args: vec![Expr::String(source.into())],
    }));

    match Interpreter::new().eval_program(&program).unwrap() {
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
fn compiler_reproduces_its_stage_zero_artifact() {
    let source = std::fs::read_to_string("compiler/main.vel").unwrap();
    let stage_one = compile(&source).unwrap();
    let compiler = load_artifact(&stage_one).unwrap();
    let stage_two = invoke_compiler(compiler, &source);

    assert_eq!(stage_two, stage_one);
}
