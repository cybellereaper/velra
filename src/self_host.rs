use crate::ast::{Expr, Program, Stmt};
use crate::runtime::RuntimeError;
use crate::{load_artifact, Error, Interpreter, Value};
use std::sync::OnceLock;

pub const EMBEDDED_COMPILER_ARTIFACT: &str = include_str!("../compiler/bootstrap.velc");

static EMBEDDED_COMPILER_PROGRAM: OnceLock<Program> = OnceLock::new();

pub struct SelfHostedCompiler {
    interpreter: Interpreter,
}

impl SelfHostedCompiler {
    pub fn new() -> Result<Self, Error> {
        let program = embedded_compiler_program()?;
        let mut interpreter = Interpreter::new();
        interpreter.eval_program(program)?;
        Ok(Self { interpreter })
    }

    pub fn compile(&mut self, source: &str) -> Result<String, Error> {
        let call = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Ident("compile".into())),
                args: vec![Expr::String(source.into())],
            })],
        };

        match self.interpreter.eval_program(&call)? {
            Value::String(artifact) => Ok(artifact),
            value => Err(RuntimeError::new(format!(
                "embedded compiler returned {}, expected String",
                value.type_name()
            ))
            .into()),
        }
    }

    pub fn check(&mut self, source: &str) -> Result<Program, Error> {
        let artifact = self.compile(source)?;
        load_artifact(&artifact)
    }
}

impl Default for SelfHostedCompiler {
    fn default() -> Self {
        Self::new().expect("embedded compiler artifact must be valid")
    }
}

fn embedded_compiler_program() -> Result<&'static Program, Error> {
    if let Some(program) = EMBEDDED_COMPILER_PROGRAM.get() {
        return Ok(program);
    }

    let program = load_artifact(EMBEDDED_COMPILER_ARTIFACT)?;
    let _ = EMBEDDED_COMPILER_PROGRAM.set(program);
    Ok(EMBEDDED_COMPILER_PROGRAM
        .get()
        .expect("embedded compiler program must be initialized"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;

    #[test]
    fn embedded_compiler_matches_stage_zero_artifact() {
        let source = include_str!("../compiler/main.vel");
        assert_eq!(EMBEDDED_COMPILER_ARTIFACT, compile(source).unwrap());
    }

    #[test]
    fn embedded_compiler_compiles_without_source_parser() {
        let source = "answer = 40 + 2\nanswer";
        let mut compiler = SelfHostedCompiler::new().unwrap();
        assert_eq!(compiler.compile(source).unwrap(), compile(source).unwrap());
    }
}
