use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;
use velra::{check, compile, load_artifact, Error, Interpreter, Value};

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
            let path = args
                .next()
                .ok_or_else(|| "usage: velra run <file.vel>".to_owned())?;
            ensure_no_extra_args(args)?;
            run_file(&path)
        }
        "check" => {
            let path = args
                .next()
                .ok_or_else(|| "usage: velra check <file.vel>".to_owned())?;
            ensure_no_extra_args(args)?;
            check_file(&path)
        }
        "compile" => {
            let input = args
                .next()
                .ok_or_else(|| "usage: velra compile <input.vel> <output.velc>".to_owned())?;
            let output = args
                .next()
                .ok_or_else(|| "usage: velra compile <input.vel> <output.velc>".to_owned())?;
            ensure_no_extra_args(args)?;
            compile_file(&input, &output)
        }
        "exec" => {
            let path = args
                .next()
                .ok_or_else(|| "usage: velra exec <file.velc>".to_owned())?;
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

fn run_file(path: &str) -> Result<(), String> {
    let source = read_source(path)?;
    let program = check(&source).map_err(|error| render_error(path, &source, &error))?;
    execute(path, program)
}

fn check_file(path: &str) -> Result<(), String> {
    let source = read_source(path)?;
    check(&source).map_err(|error| render_error(path, &source, &error))?;
    println!("{path}: ok");
    Ok(())
}

fn compile_file(input: &str, output: &str) -> Result<(), String> {
    let source = read_source(input)?;
    let artifact = compile(&source).map_err(|error| render_error(input, &source, &error))?;
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

        match check(&line) {
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
        Error::Runtime(error) => return format!("{path}: runtime error: {error}"),
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
Commands:\n  run      Execute a Velra source file\n  check    Lex and parse without executing\n  compile  Compile source to a Velra artifact\n  exec     Execute a compiled Velra artifact\n  repl     Start an interactive session\n"
    );
}
