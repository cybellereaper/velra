use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;
use velra::{load_artifact, Error, Interpreter, SelfHostedCompiler, Value};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match run_cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => { print_help(); Ok(()) }
        Some("-h") | Some("--help") | Some("help") => { print_help(); Ok(()) }
        Some("-V") | Some("--version") | Some("version") => { println!("velra {VERSION}"); Ok(()) }
        Some("run") | Some("check") | Some("compile") | Some("exec") | Some("repl") => {
            dispatch_command(args.next(), args)
        }
        Some(path) if !path.starts_with('-') => run_file(path),
        Some(command) => Err(format!("unknown command '{command}'. Run 'velra --help'.")),
    }
}

fn dispatch_command(command_arg: Option<String>, mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let command = command_arg.as_deref().ok_or_else(|| "missing command".to_owned())?;
    match command {
        "run" | "check" => {
            let path = args.next().ok_or_else(|| format!("usage: velra {command} <file.vel>"))?;
            ensure_no_extra_args(args)?;
            if command == "run" { run_file(&path) } else { check_file(&path) }
        }
        "compile" => {
            let input = args.next().ok_or_else(|| "usage: velra compile <input.vel> <output.velc>".to_owned())?;
            let output = args.next().ok_or_else(|| "usage: velra compile <input.vel> <output.velc>".to_owned())?;
            ensure_no_extra_args(args)?;
            compile_file(&input, &output)
        }
        "exec" => {
            let path = args.next().ok_or_else(|| "usage: velra exec <file.velc>".to_owned())?;
            ensure_no_extra_args(args)?;
            exec_file(&path)
        }
        "repl" => { ensure_no_extra_args(args)?; repl() }
        _ => unreachable!(),
    }
}

fn compiler() -> Result<SelfHostedCompiler, String> {
    SelfHostedCompiler::new().map_err(|e| format!("failed to initialize embedded Velra compiler: {e}"))
}

fn run_file(path: &str) -> Result<(), String> {
    let source = read_source(path)?;
    let mut compiler = compiler()?;
    let program = compiler.check(&source).map_err(|e| render_error(path, &source, &e))?;
    execute(path, program)
}

fn check_file(path: &str) -> Result<(), String> {
    let source = read_source(path)?;
    let mut compiler = compiler()?;
    compiler.check(&source).map_err(|e| render_error(path, &source, &e))?;
    println!("{path}: ok");
    Ok(())
}

fn compile_file(input: &str, output: &str) -> Result<(), String> {
    let source = read_source(input)?;
    let mut compiler = compiler()?;
    let artifact = compiler.compile(&source).map_err(|e| render_error(input, &source, &e))?;
    fs::write(output, artifact).map_err(|e| format!("failed to write '{output}': {e}"))
}

fn exec_file(path: &str) -> Result<(), String> {
    let artifact = read_source(path)?;
    let program = load_artifact(&artifact).map_err(|e| render_error(path, &artifact, &e))?;
    execute(path, program)
}

fn execute(path: &str, program: velra::ast::Program) -> Result<(), String> {
    Interpreter::new().eval_program(&program).map(|_| ()).map_err(|e| format!("{path}: runtime error: {e}"))
}

fn repl() -> Result<(), String> {
    let stdin = io::stdin();
    let mut compiler = compiler()?;
    let mut interpreter = Interpreter::new();
    let mut line = String::new();
    println!("Velra {VERSION}. Ctrl-D to exit.");
    loop {
        print!("> ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        line.clear();
        if stdin.read_line(&mut line).map_err(|e| e.to_string())? == 0 { break; }
        if line.trim().is_empty() { continue; }
        match compiler.check(&line) {
            Ok(program) => match interpreter.eval_program(&program) {
                Ok(Value::Null) => {}
                Ok(value) => println!("{value}"),
                Err(e) => eprintln!("runtime error: {e}"),
            },
            Err(e) => eprintln!("{}", render_error("<repl>", &line, &e)),
        }
    }
    Ok(())
}

fn read_source(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("failed to read '{path}': {e}"))
}

fn ensure_no_extra_args(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    match args.next() { Some(arg) => Err(format!("unexpected argument '{arg}'")), None => Ok(()) }
}

fn render_error(path: &str, source: &str, error: &Error) -> String {
    let (message, offset) = match error {
        Error::Artifact(e) => (&e.message, e.offset),
        Error::Lex(e) => (&e.message, e.span.start),
        Error::Parse(e) => (&e.message, e.offset),
        Error::Runtime(e) => return format!("{path}: compile/runtime error: {e}"),
    };
    let (line, column) = line_column(source, offset);
    format!("{path}:{line}:{column}: {message}")
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    (prefix.bytes().filter(|b| *b == b'\n').count() + 1,
     prefix.rsplit_once('\n').map(|(_, s)| s.chars().count() + 1).unwrap_or_else(|| prefix.chars().count() + 1))
}

fn print_help() {
    println!("Velra {VERSION}\n\nUsage:\n  velra <file.vel>\n  velra run <file.vel>\n  velra check <file.vel>\n  velra compile <input.vel> <output.velc>\n  velra exec <file.velc>\n  velra repl");
}
