pub mod artifact;
pub mod ast;
pub mod lexer;
pub mod parser;
pub mod runtime;

use std::fmt;

pub use parser::parse;
pub use runtime::{Interpreter, RuntimeError, Value};

#[derive(Debug)]
pub enum Error {
    Artifact(artifact::ArtifactError),
    Lex(lexer::LexError),
    Parse(parser::ParseError),
    Runtime(RuntimeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Artifact(error) => error.fmt(f),
            Self::Lex(error) => error.fmt(f),
            Self::Parse(error) => error.fmt(f),
            Self::Runtime(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<artifact::ArtifactError> for Error {
    fn from(value: artifact::ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<lexer::LexError> for Error {
    fn from(value: lexer::LexError) -> Self {
        Self::Lex(value)
    }
}

impl From<parser::ParseError> for Error {
    fn from(value: parser::ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<RuntimeError> for Error {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

pub fn check(source: &str) -> Result<ast::Program, Error> {
    let tokens = lexer::lex(source)?;
    Ok(parser::Parser::new(tokens).parse_program()?)
}

pub fn load_artifact(source: &str) -> Result<ast::Program, Error> {
    Ok(artifact::decode(source)?)
}

pub fn run(source: &str) -> Result<Value, Error> {
    let program = check(source)?;
    Ok(Interpreter::new().eval_program(&program)?)
}
