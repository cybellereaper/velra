use crate::ast::{
    BinaryOp, Block, ElseBranch, Expr, FunctionBody, Param, Program, Stmt, UnaryOp, WhenBody,
    WhenCase,
};
use std::fmt::Write;

const HEADER: &str = "VELRA-AST-1\n";

pub fn encode(program: &Program) -> String {
    let mut output = String::from(HEADER);
    encode_program(&mut output, program);
    output
}

fn encode_program(output: &mut String, program: &Program) {
    output.push_str("(program");
    for statement in &program.statements {
        output.push(' ');
        encode_stmt(output, statement);
    }
    output.push(')');
}

fn encode_stmt(output: &mut String, statement: &Stmt) {
    match statement {
        Stmt::Use { path } => {
            output.push_str("(use");
            for part in path {
                output.push(' ');
                encode_string(output, part);
            }
            output.push(')');
        }
        Stmt::Var {
            name,
            type_name,
            value,
        } => {
            output.push_str("(var ");
            encode_string(output, name);
            output.push(' ');
            encode_optional_string(output, type_name.as_deref());
            output.push(' ');
            encode_expr(output, value);
            output.push(')');
        }
        Stmt::Assign { target, value } => {
            output.push_str("(assign ");
            encode_expr(output, target);
            output.push(' ');
            encode_expr(output, value);
            output.push(')');
        }
        Stmt::Return(value) => {
            output.push_str("(return ");
            encode_optional_expr(output, value.as_ref());
            output.push(')');
        }
        Stmt::For {
            name,
            iterable,
            body,
        } => {
            output.push_str("(for ");
            encode_string(output, name);
            output.push(' ');
            encode_expr(output, iterable);
            output.push(' ');
            encode_block(output, body);
            output.push(')');
        }
        Stmt::Function(function) => {
            output.push_str("(function ");
            encode_string(output, &function.name);
            output.push(' ');
            encode_params(output, &function.params);
            output.push(' ');
            match &function.body {
                FunctionBody::Expr(expr) => {
                    output.push_str("(body-expr ");
                    encode_expr(output, expr);
                    output.push(')');
                }
                FunctionBody::Block(block) => {
                    output.push_str("(body-block ");
                    encode_block(output, block);
                    output.push(')');
                }
            }
            output.push(')');
        }
        Stmt::Data(data) => {
            output.push_str("(data ");
            encode_string(output, &data.name);
            output.push(' ');
            encode_params(output, &data.params);
            output.push_str(" (computed");
            for (name, expr) in &data.computed {
                output.push_str(" (field ");
                encode_string(output, name);
                output.push(' ');
                encode_expr(output, expr);
                output.push(')');
            }
            output.push_str("))");
        }
        Stmt::Expr(expr) => {
            output.push_str("(expr ");
            encode_expr(output, expr);
            output.push(')');
        }
        Stmt::Pub(statement) => {
            output.push_str("(pub ");
            encode_stmt(output, statement);
            output.push(')');
        }
    }
}

fn encode_block(output: &mut String, block: &Block) {
    output.push_str("(block");
    for statement in &block.statements {
        output.push(' ');
        encode_stmt(output, statement);
    }
    output.push(')');
}

fn encode_params(output: &mut String, params: &[Param]) {
    output.push_str("(params");
    for param in params {
        output.push_str(" (param ");
        encode_string(output, &param.name);
        output.push(' ');
        encode_optional_string(output, param.type_name.as_deref());
        output.push(')');
    }
    output.push(')');
}

fn encode_expr(output: &mut String, expr: &Expr) {
    match expr {
        Expr::Null => output.push_str("null"),
        Expr::Bool(value) => write!(output, "(bool {value})").expect("write to String cannot fail"),
        Expr::Int(value) => write!(output, "(int {value})").expect("write to String cannot fail"),
        Expr::Float(value) => {
            write!(output, "(float {value})").expect("write to String cannot fail")
        }
        Expr::String(value) => {
            output.push_str("(string ");
            encode_string(output, value);
            output.push(')');
        }
        Expr::Ident(name) => {
            output.push_str("(ident ");
            encode_string(output, name);
            output.push(')');
        }
        Expr::List(items) => {
            output.push_str("(list");
            for item in items {
                output.push(' ');
                encode_expr(output, item);
            }
            output.push(')');
        }
        Expr::Unary { op, expr } => {
            output.push_str("(unary ");
            output.push_str(unary_name(*op));
            output.push(' ');
            encode_expr(output, expr);
            output.push(')');
        }
        Expr::Binary { left, op, right } => {
            output.push_str("(binary ");
            output.push_str(binary_name(*op));
            output.push(' ');
            encode_expr(output, left);
            output.push(' ');
            encode_expr(output, right);
            output.push(')');
        }
        Expr::Call { callee, args } => {
            output.push_str("(call ");
            encode_expr(output, callee);
            output.push_str(" (args");
            for arg in args {
                output.push(' ');
                encode_expr(output, arg);
            }
            output.push_str("))");
        }
        Expr::Member { object, name, safe } => {
            output.push_str("(member ");
            encode_expr(output, object);
            output.push(' ');
            encode_string(output, name);
            write!(output, " {safe})").expect("write to String cannot fail");
        }
        Expr::Index { object, index } => {
            output.push_str("(index ");
            encode_expr(output, object);
            output.push(' ');
            encode_expr(output, index);
            output.push(')');
        }
        Expr::Propagate { expr } => {
            output.push_str("(propagate ");
            encode_expr(output, expr);
            output.push(')');
        }
        Expr::Lambda { params, body } => {
            output.push_str("(lambda ");
            encode_params(output, params);
            output.push(' ');
            encode_expr(output, body);
            output.push(')');
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            output.push_str("(if ");
            encode_expr(output, condition);
            output.push(' ');
            encode_block(output, then_branch);
            output.push(' ');
            encode_else(output, else_branch.as_ref());
            output.push(')');
        }
        Expr::When { subject, cases } => {
            output.push_str("(when ");
            encode_optional_expr(output, subject.as_deref());
            output.push_str(" (cases");
            for case in cases {
                output.push(' ');
                encode_when_case(output, case);
            }
            output.push_str("))");
        }
    }
}

fn encode_else(output: &mut String, branch: Option<&ElseBranch>) {
    match branch {
        None => output.push_str("(none)"),
        Some(ElseBranch::Block(block)) => {
            output.push_str("(else-block ");
            encode_block(output, block);
            output.push(')');
        }
        Some(ElseBranch::If(expr)) => {
            output.push_str("(else-if ");
            encode_expr(output, expr);
            output.push(')');
        }
    }
}

fn encode_when_case(output: &mut String, case: &WhenCase) {
    output.push_str("(case (patterns");
    for pattern in &case.patterns {
        output.push(' ');
        encode_expr(output, pattern);
    }
    output.push_str(") ");
    encode_optional_expr(output, case.guard.as_ref());
    output.push(' ');
    match &case.body {
        WhenBody::Expr(expr) => {
            output.push_str("(when-expr ");
            encode_expr(output, expr);
            output.push(')');
        }
        WhenBody::Block(block) => {
            output.push_str("(when-block ");
            encode_block(output, block);
            output.push(')');
        }
    }
    write!(output, " {})", case.is_else).expect("write to String cannot fail");
}

fn encode_optional_expr(output: &mut String, expr: Option<&Expr>) {
    match expr {
        Some(expr) => {
            output.push_str("(some ");
            encode_expr(output, expr);
            output.push(')');
        }
        None => output.push_str("(none)"),
    }
}

fn encode_optional_string(output: &mut String, value: Option<&str>) {
    match value {
        Some(value) => {
            output.push_str("(some ");
            encode_string(output, value);
            output.push(')');
        }
        None => output.push_str("(none)"),
    }
}

fn encode_string(output: &mut String, value: &str) {
    output.push('"');
    for ch in value.chars() {
        match ch {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\0' => output.push_str("\\0"),
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            ch => output.push(ch),
        }
    }
    output.push('"');
}

fn unary_name(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "negate",
        UnaryOp::Not => "not",
    }
}

fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Elvis => "elvis",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Equal => "equal",
        BinaryOp::NotEqual => "not-equal",
        BinaryOp::Less => "less",
        BinaryOp::LessEqual => "less-equal",
        BinaryOp::Greater => "greater",
        BinaryOp::GreaterEqual => "greater-equal",
        BinaryOp::Add => "add",
        BinaryOp::Subtract => "subtract",
        BinaryOp::Multiply => "multiply",
        BinaryOp::Divide => "divide",
        BinaryOp::Remainder => "remainder",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{artifact, check};

    #[test]
    fn artifact_round_trips_program() {
        let source = r#"
add(a: Int, b: Int) => a + b
value = if true { add(1, 2) } else { 0 }
when value {
    3 => "ok"
    else => "bad"
}
"#;
        let program = check(source).unwrap();
        let artifact = encode(&program);
        assert_eq!(artifact::decode(&artifact).unwrap(), program);
    }
}
