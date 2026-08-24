use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;
use velra::{load_artifact, Error, Interpreter, SelfHostedCompiler, Value};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match run_cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        "-V" | "--version" | "version" => {
            println!("velra {VERSION}");
            Ok(())
        }
        "run" => {
            let path = required_arg(&mut args, "usage: velra run <file.vel>")?;
            ensure_no_extra_args(args)?;
            run_file(&path)
        }
        "check" => {
            let path = required_arg(&mut args, "usage: velra check <file.vel>")?;
            ensure_no_extra_args(args)?;
            check_file(&path)
        }
        "compile" => {
            let usage = "usage: velra compile <input.vel> <output.velc>";
            let input = required_arg(&mut args, usage)?;
            let output = required_arg(&mut args, usage)?;
            ensure_no_extra_args(args)?;
            compile_file(&input, &output)
        }
        "exec" => {
            let path = required_arg(&mut args, "usage: velra exec <file.velc>")?;
            ensure_no_extra_args(args)?;
            exec_file(&path)
        }
        "repl" => {
            ensure_no_extra_args(args)?;
            repl()
        }
        path if !path.starts_with('-') => {
            ensure_no_extra_args(args)?;
            run_file(path)
        }
        other => Err(format!("unknown command '{other}'. Run 'velra --help'.")),
    }
}

fn self_hosted_compiler() -> Result<SelfHostedCompiler, String> {
    SelfHostedCompiler::new()
        .map_err(|error| format!("failed to initialize embedded Velra compiler: {error}"))
}

fn run_file(path: &str) -> Result<(), String> {
    let source = read_source(path)?;
    let mut compiler = self_hosted_compiler()?;
    let program = compiler
        .check(&source)
        .map_err(|error| render_error(path, &source, &error))?;
    execute(path, program)
}

fn check_file(path: &str) -> Result<(), String> {
    let source = read_source(path)?;
    let mut compiler = self_hosted_compiler()?;
    compiler
        .check(&source)
        .map_err(|error| render_error(path, &source, &error))?;
    println!("{path}: ok");
    Ok(())
}

fn compile_file(input: &str, output: &str) -> Result<(), String> {
    let source = read_source(input)?;
    let mut compiler = self_hosted_compiler()?;
    let artifact = compiler
        .compile(&source)
        .map_err(|error| render_error(input, &source, &error))?;
    fs::write(output, artifact).map_err(|error| format!("failed to write '{output}': {error}"))
}

fn exec_file(path: &str) -> Result<(), String> {
    let artifact = read_source(path)?;
    let program =
        load_artifact(&artifact).map_err(|error| render_error(path, &artifact, &error))?;
    execute(path, program)
}

fn execute(path: &str, program: velra::ast::Program) -> Result<(), String> {
    Interpreter::new()
        .eval_program(&program)
        .map(|_| ())
        .map_err(|error| format!("{path}: runtime error: {error}"))
}

fn repl() -> Result<(), String> {
    let stdin = io::stdin();
    let mut compiler = self_hosted_compiler()?;
    let mut interpreter = Interpreter::new();
    let mut line = String::new();

    println!("Velra {VERSION}. Ctrl-D to exit.");
    loop {
        print!("> ");
        io::stdout()
            .flush()
            .map_err(|error| format!("failed to write prompt: {error}"))?;
        line.clear();
        let read = stdin
            .read_line(&mut line)
            .map_err(|error| format!("failed to read input: {error}"))?;
        if read == 0 {
            println!();
            break;
        }
        if line.trim().is_empty() {
            continue;
        }

        match compiler.check(&line) {
            Ok(program) => match interpreter.eval_program(&program) {
                Ok(Value::Null) => {}
                Ok(value) => println!("{value}"),
                Err(error) => eprintln!("runtime error: {error}"),
            },
            Err(error) => eprintln!("{}", render_error("<repl>", &line, &error)),
        }
    }
    Ok(())
}

fn read_source(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read '{path}': {error}"))
}

fn required_arg(
    args: &mut impl Iterator<Item = String>,
    usage: &str,
) -> Result<String, String> {
    args.next().ok_or_else(|| usage.to_owned())
}

fn ensure_no_extra_args(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    if let Some(arg) = args.next() {
        Err(format!("unexpected argument '{arg}'"))
    } else {
        Ok(())
    }
}

fn render_error(path: &str, source: &str, error: &Error) -> String {
    let (message, offset) = match error {
        Error::Artifact(error) => (error.message.as_str(), error.offset),
        Error::Lex(error) => (error.message.as_str(), error.span.start),
        Error::Parse(error) => (error.message.as_str(), error.offset),
        Error::Runtime(error) => return format!("{path}: compile/runtime error: {error}"),
    };
    let (line, column) = line_column(source, offset);
    format!("{path}:{line}:{column}: {message}")
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map(|(_, tail)| tail.chars().count() + 1)
        .unwrap_or_else(|| prefix.chars().count() + 1);
    (line, column)
}

fn print_help() {
    println!(
        "Velra {VERSION}\n\n\
Usage:\n  velra <file.vel>\n  velra run <file.vel>\n  velra check <file.vel>\n  velra compile <input.vel> <output.velc>\n  velra exec <file.velc>\n  velra repl\n\n\
Commands:\n  run      Compile with the embedded Velra compiler and execute source\n  check    Compile with the embedded Velra compiler without executing\n  compile  Compile source to a Velra artifact using the Velra compiler\n  exec     Execute a compiled Velra artifact\n  repl     Start an interactive session backed by the Velra compiler\n"
    );
}
