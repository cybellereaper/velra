use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;
use velra::{
    check, compile, lexer, load_artifact, Interpreter, SelfHostedCompiler,
    EMBEDDED_COMPILER_ARTIFACT,
};

fn large_source(lines: usize) -> String {
    let mut source = String::with_capacity(lines * 24);
    source.push_str("v0 = 0\n");
    for index in 1..lines {
        writeln!(&mut source, "v{index} = v{} + 1", index - 1).unwrap();
    }
    writeln!(&mut source, "v{}", lines - 1).unwrap();
    source
}

fn measure<T>(name: &str, iterations: usize, mut run: impl FnMut() -> T) {
    let started = Instant::now();
    for _ in 0..iterations {
        black_box(run());
    }
    let elapsed = started.elapsed();
    eprintln!(
        "{name}: {iterations} iterations in {:.3} ms ({:.3} ms/iter)",
        elapsed.as_secs_f64() * 1_000.0,
        elapsed.as_secs_f64() * 1_000.0 / iterations as f64
    );
}

fn uncached_self_hosted_init() -> Interpreter {
    let program = load_artifact(EMBEDDED_COMPILER_ARTIFACT).unwrap();
    let mut interpreter = Interpreter::new();
    interpreter.eval_program(&program).unwrap();
    interpreter
}

#[test]
#[ignore = "manual performance test"]
fn lex_large_source() {
    let source = large_source(2_000);
    measure("lex 2k statements", 100, || {
        lexer::lex(black_box(&source)).unwrap()
    });
}

#[test]
#[ignore = "manual performance test"]
fn check_large_source() {
    let source = large_source(2_000);
    measure("check 2k statements", 20, || {
        check(black_box(&source)).unwrap()
    });
}

#[test]
#[ignore = "manual performance test"]
fn compile_large_source() {
    let source = large_source(2_000);
    measure("compile 2k statements", 10, || {
        compile(black_box(&source)).unwrap()
    });
}

#[test]
#[ignore = "manual performance test"]
fn self_hosted_check_medium_source() {
    let source = large_source(100);
    let mut compiler = SelfHostedCompiler::new().unwrap();
    measure("self-hosted check 100 statements", 3, || {
        compiler.check(black_box(&source)).unwrap()
    });
}

#[test]
#[ignore = "manual performance test"]
fn self_hosted_initialization() {
    let _ = SelfHostedCompiler::new().unwrap();
    measure("self-hosted init uncached", 10, uncached_self_hosted_init);
    measure("self-hosted init cached", 10, || {
        SelfHostedCompiler::new().unwrap()
    });
}
