use crate::ast::{
    BinaryOp, Block, DataDecl, ElseBranch, EnumDecl, EnumVariantDecl, Expr, FunctionBody,
    FunctionDecl, Param, Program, Stmt, UnaryOp, WhenBody, WhenCase,
};
use std::fmt;

const HEADER: &str = "VELRA-AST-1\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError {
    pub message: String,
    pub offset: usize,
}

impl ArtifactError {
    fn new(message: impl Into<String>, offset: usize) -> Self {
        Self {
            message: message.into(),
            offset,
        }
    }
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for ArtifactError {}

#[derive(Debug, Clone, PartialEq)]
enum Form {
    Atom(String),
    String(String),
    List(Vec<Form>),
}

pub fn decode(source: &str) -> Result<Program, ArtifactError> {
    let body = source
        .strip_prefix(HEADER)
        .ok_or_else(|| ArtifactError::new("invalid Velra artifact header", 0))?;
    let mut parser = FormParser::new(body, HEADER.len());
    let form = parser.parse_form()?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return Err(parser.error("unexpected trailing artifact data"));
    }
    decode_program(&form)
}

struct FormParser<'a> {
    source: &'a str,
    cursor: usize,
    offset_base: usize,
}

impl<'a> FormParser<'a> {
    fn new(source: &'a str, offset_base: usize) -> Self {
        Self {
            source,
            cursor: 0,
            offset_base,
        }
    }

    fn parse_form(&mut self) -> Result<Form, ArtifactError> {
        self.skip_whitespace();
        match self.current_char() {
            Some('(') => self.parse_list(),
            Some('"') => self.parse_string(),
            Some(_) => self.parse_atom(),
            None => Err(self.error("expected artifact form")),
        }
    }

    fn parse_list(&mut self) -> Result<Form, ArtifactError> {
        self.cursor += 1;
        let mut forms = Vec::new();
        loop {
            self.skip_whitespace();
            match self.current_char() {
                Some(')') => {
                    self.cursor += 1;
                    return Ok(Form::List(forms));
                }
                Some(_) => forms.push(self.parse_form()?),
                None => return Err(self.error("unterminated artifact list")),
            }
        }
    }

    fn parse_string(&mut self) -> Result<Form, ArtifactError> {
        self.cursor += 1;
        let mut value = String::new();
        while let Some(ch) = self.current_char() {
            match ch {
                '"' => {
                    self.cursor += 1;
                    return Ok(Form::String(value));
                }
                '\\' => {
                    self.cursor += 1;
                    let escape = self
                        .current_char()
                        .ok_or_else(|| self.error("unterminated artifact string escape"))?;
                    value.push(match escape {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '0' => '\0',
                        '"' => '"',
                        '\\' => '\\',
                        other => {
                            return Err(self
                                .error(format!("unsupported artifact string escape '\\{other}'")))
                        }
                    });
                    self.cursor += escape.len_utf8();
                }
                other => {
                    value.push(other);
                    self.cursor += other.len_utf8();
                }
            }
        }
        Err(self.error("unterminated artifact string"))
    }

    fn parse_atom(&mut self) -> Result<Form, ArtifactError> {
        let start = self.cursor;
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() || ch == '(' || ch == ')' {
                break;
            }
            self.cursor += ch.len_utf8();
        }
        if start == self.cursor {
            return Err(self.error("expected artifact atom"));
        }
        Ok(Form::Atom(self.source[start..self.cursor].to_owned()))
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.cursor += ch.len_utf8();
        }
    }

    fn current_char(&self) -> Option<char> {
        self.source.get(self.cursor..)?.chars().next()
    }

    fn is_eof(&self) -> bool {
        self.cursor >= self.source.len()
    }

    fn error(&self, message: impl Into<String>) -> ArtifactError {
        ArtifactError::new(message, self.offset_base + self.cursor)
    }
}

fn decode_program(form: &Form) -> Result<Program, ArtifactError> {
    let list = expect_list(form, "program")?;
    expect_tag(list, "program")?;
    let mut statements = Vec::with_capacity(list.len().saturating_sub(1));
    for form in &list[1..] {
        statements.push(decode_stmt(form)?);
    }
    Ok(Program { statements })
}

fn decode_stmt(form: &Form) -> Result<Stmt, ArtifactError> {
    let list = expect_nonempty_list(form, "statement")?;
    match atom(&list[0])? {
        "use" => {
            let mut path = Vec::with_capacity(list.len().saturating_sub(1));
            for part in &list[1..] {
                path.push(string(part)?.to_owned());
            }
            Ok(Stmt::Use { path })
        }
        "var" => {
            expect_len(list, 4, "var")?;
            Ok(Stmt::Var {
                name: string(&list[1])?.to_owned(),
                type_name: decode_optional_string(&list[2])?,
                value: decode_expr(&list[3])?,
            })
        }
        "assign" => {
            expect_len(list, 3, "assign")?;
            Ok(Stmt::Assign {
                target: decode_expr(&list[1])?,
                value: decode_expr(&list[2])?,
            })
        }
        "return" => {
            expect_len(list, 2, "return")?;
            Ok(Stmt::Return(decode_optional_expr(&list[1])?))
        }
        "while" => {
            expect_len(list, 3, "while")?;
            Ok(Stmt::While {
                condition: decode_expr(&list[1])?,
                body: decode_block(&list[2])?,
            })
        }
        "break" => {
            expect_len(list, 1, "break")?;
            Ok(Stmt::Break)
        }
        "continue" => {
            expect_len(list, 1, "continue")?;
            Ok(Stmt::Continue)
        }
        "for" => {
            expect_len(list, 4, "for")?;
            Ok(Stmt::For {
                name: string(&list[1])?.to_owned(),
                iterable: decode_expr(&list[2])?,
                body: decode_block(&list[3])?,
            })
        }
        "function" => {
            expect_len(list, 4, "function")?;
            Ok(Stmt::Function(FunctionDecl {
                name: string(&list[1])?.to_owned(),
                params: decode_params(&list[2])?,
                body: decode_function_body(&list[3])?,
            }))
        }
        "data" => {
            expect_len(list, 4, "data")?;
            Ok(Stmt::Data(DataDecl {
                name: string(&list[1])?.to_owned(),
                params: decode_params(&list[2])?,
                computed: decode_computed(&list[3])?,
            }))
        }
        "enum" => {
            expect_len(list, 3, "enum")?;
            let variants = expect_nonempty_list(&list[2], "enum variants")?;
            expect_tag(variants, "variants")?;
            let mut decoded = Vec::with_capacity(variants.len().saturating_sub(1));
            for form in &variants[1..] {
                let variant = expect_nonempty_list(form, "enum variant")?;
                expect_tag(variant, "variant")?;
                expect_len(variant, 3, "enum variant")?;
                decoded.push(EnumVariantDecl {
                    name: string(&variant[1])?.to_owned(),
                    params: decode_params(&variant[2])?,
                });
            }
            Ok(Stmt::Enum(EnumDecl {
                name: string(&list[1])?.to_owned(),
                variants: decoded,
            }))
        }
        "expr" => {
            expect_len(list, 2, "expr")?;
            Ok(Stmt::Expr(decode_expr(&list[1])?))
        }
        "pub" => {
            expect_len(list, 2, "pub")?;
            Ok(Stmt::Pub(Box::new(decode_stmt(&list[1])?)))
        }
        tag => invalid(format!("unknown statement tag '{tag}'")),
    }
}

fn decode_block(form: &Form) -> Result<Block, ArtifactError> {
    let list = expect_nonempty_list(form, "block")?;
    expect_tag(list, "block")?;
    let mut statements = Vec::with_capacity(list.len().saturating_sub(1));
    for statement in &list[1..] {
        statements.push(decode_stmt(statement)?);
    }
    Ok(Block { statements })
}

fn decode_params(form: &Form) -> Result<Vec<Param>, ArtifactError> {
    let list = expect_nonempty_list(form, "params")?;
    expect_tag(list, "params")?;
    let mut params = Vec::with_capacity(list.len().saturating_sub(1));
    for form in &list[1..] {
        let param = expect_nonempty_list(form, "param")?;
        expect_tag(param, "param")?;
        expect_len(param, 3, "param")?;
        params.push(Param {
            name: string(&param[1])?.to_owned(),
            type_name: decode_optional_string(&param[2])?,
        });
    }
    Ok(params)
}

fn decode_computed(form: &Form) -> Result<Vec<(String, Expr)>, ArtifactError> {
    let list = expect_nonempty_list(form, "computed fields")?;
    expect_tag(list, "computed")?;
    let mut fields = Vec::with_capacity(list.len().saturating_sub(1));
    for form in &list[1..] {
        let field = expect_nonempty_list(form, "computed field")?;
        expect_tag(field, "field")?;
        expect_len(field, 3, "computed field")?;
        fields.push((string(&field[1])?.to_owned(), decode_expr(&field[2])?));
    }
    Ok(fields)
}

fn decode_function_body(form: &Form) -> Result<FunctionBody, ArtifactError> {
    let list = expect_nonempty_list(form, "function body")?;
    expect_len(list, 2, "function body")?;
    match atom(&list[0])? {
        "body-expr" => Ok(FunctionBody::Expr(decode_expr(&list[1])?)),
        "body-block" => Ok(FunctionBody::Block(decode_block(&list[1])?)),
        tag => invalid(format!("unknown function body tag '{tag}'")),
    }
}

fn decode_expr(form: &Form) -> Result<Expr, ArtifactError> {
    if let Form::Atom(value) = form {
        if value == "null" {
            return Ok(Expr::Null);
        }
    }

    let list = expect_nonempty_list(form, "expression")?;
    match atom(&list[0])? {
        "bool" => {
            expect_len(list, 2, "bool")?;
            Ok(Expr::Bool(match atom(&list[1])? {
                "true" => true,
                "false" => false,
                value => return invalid(format!("invalid bool '{value}'")),
            }))
        }
        "int" => {
            expect_len(list, 2, "int")?;
            let value = atom(&list[1])?
                .parse()
                .map_err(|_| ArtifactError::new("invalid Int artifact value", 0))?;
            Ok(Expr::Int(value))
        }
        "float" => {
            expect_len(list, 2, "float")?;
            let value = atom(&list[1])?
                .parse()
                .map_err(|_| ArtifactError::new("invalid Float artifact value", 0))?;
            Ok(Expr::Float(value))
        }
        "string" => {
            expect_len(list, 2, "string")?;
            Ok(Expr::String(string(&list[1])?.to_owned()))
        }
        "ident" => {
            expect_len(list, 2, "ident")?;
            Ok(Expr::Ident(string(&list[1])?.to_owned()))
        }
        "list" => {
            let mut items = Vec::with_capacity(list.len().saturating_sub(1));
            for item in &list[1..] {
                items.push(decode_expr(item)?);
            }
            Ok(Expr::List(items))
        }
        "map" => {
            let mut entries = Vec::with_capacity(list.len().saturating_sub(1));
            for form in &list[1..] {
                let entry = expect_nonempty_list(form, "map entry")?;
                expect_tag(entry, "entry")?;
                expect_len(entry, 3, "map entry")?;
                entries.push((decode_expr(&entry[1])?, decode_expr(&entry[2])?));
            }
            Ok(Expr::Map(entries))
        }
        "set" => {
            let mut items = Vec::with_capacity(list.len().saturating_sub(1));
            for item in &list[1..] {
                items.push(decode_expr(item)?);
            }
            Ok(Expr::Set(items))
        }
        "rest" => {
            expect_len(list, 2, "rest")?;
            Ok(Expr::Rest(string(&list[1])?.to_owned()))
        }
        "unary" => {
            expect_len(list, 3, "unary")?;
            Ok(Expr::Unary {
                op: decode_unary(atom(&list[1])?)?,
                expr: Box::new(decode_expr(&list[2])?),
            })
        }
        "binary" => {
            expect_len(list, 4, "binary")?;
            Ok(Expr::Binary {
                op: decode_binary(atom(&list[1])?)?,
                left: Box::new(decode_expr(&list[2])?),
                right: Box::new(decode_expr(&list[3])?),
            })
        }
        "call" => {
            expect_len(list, 3, "call")?;
            let args = expect_nonempty_list(&list[2], "args")?;
            expect_tag(args, "args")?;
            let mut decoded = Vec::with_capacity(args.len().saturating_sub(1));
            for arg in &args[1..] {
                decoded.push(decode_expr(arg)?);
            }
            Ok(Expr::Call {
                callee: Box::new(decode_expr(&list[1])?),
                args: decoded,
            })
        }
        "member" => {
            expect_len(list, 4, "member")?;
            let safe = match atom(&list[3])? {
                "true" => true,
                "false" => false,
                value => return invalid(format!("invalid member safety flag '{value}'")),
            };
            Ok(Expr::Member {
                object: Box::new(decode_expr(&list[1])?),
                name: string(&list[2])?.to_owned(),
                safe,
            })
        }
        "index" => {
            expect_len(list, 3, "index")?;
            Ok(Expr::Index {
                object: Box::new(decode_expr(&list[1])?),
                index: Box::new(decode_expr(&list[2])?),
            })
        }
        "propagate" => {
            expect_len(list, 2, "propagate")?;
            Ok(Expr::Propagate {
                expr: Box::new(decode_expr(&list[1])?),
            })
        }
        "lambda" => {
            expect_len(list, 3, "lambda")?;
            Ok(Expr::Lambda {
                params: decode_params(&list[1])?,
                body: Box::new(decode_expr(&list[2])?),
            })
        }
        "if" => {
            expect_len(list, 4, "if")?;
            Ok(Expr::If {
                condition: Box::new(decode_expr(&list[1])?),
                then_branch: decode_block(&list[2])?,
                else_branch: decode_else(&list[3])?,
            })
        }
        "when" => {
            expect_len(list, 4, "when")?;
            let cases = expect_nonempty_list(&list[3], "when cases")?;
            expect_tag(cases, "cases")?;
            let mut decoded = Vec::with_capacity(cases.len().saturating_sub(1));
            for case in &cases[1..] {
                decoded.push(decode_when_case(case)?);
            }
            Ok(Expr::When {
                binding: decode_optional_string(&list[1])?,
                subject: decode_optional_expr(&list[2])?.map(Box::new),
                cases: decoded,
            })
        }
        tag => invalid(format!("unknown expression tag '{tag}'")),
    }
}

fn decode_else(form: &Form) -> Result<Option<ElseBranch>, ArtifactError> {
    let list = expect_nonempty_list(form, "else branch")?;
    match atom(&list[0])? {
        "none" => {
            expect_len(list, 1, "none")?;
            Ok(None)
        }
        "else-block" => {
            expect_len(list, 2, "else-block")?;
            Ok(Some(ElseBranch::Block(decode_block(&list[1])?)))
        }
        "else-if" => {
            expect_len(list, 2, "else-if")?;
            Ok(Some(ElseBranch::If(Box::new(decode_expr(&list[1])?))))
        }
        tag => invalid(format!("unknown else branch tag '{tag}'")),
    }
}

fn decode_when_case(form: &Form) -> Result<WhenCase, ArtifactError> {
    let list = expect_nonempty_list(form, "when case")?;
    expect_tag(list, "case")?;
    expect_len(list, 5, "when case")?;
    let patterns = expect_nonempty_list(&list[1], "patterns")?;
    expect_tag(patterns, "patterns")?;
    let mut decoded_patterns = Vec::with_capacity(patterns.len().saturating_sub(1));
    for pattern in &patterns[1..] {
        decoded_patterns.push(decode_expr(pattern)?);
    }
    let is_else = match atom(&list[4])? {
        "true" => true,
        "false" => false,
        value => return invalid(format!("invalid when else flag '{value}'")),
    };
    Ok(WhenCase {
        patterns: decoded_patterns,
        guard: decode_optional_expr(&list[2])?,
        body: decode_when_body(&list[3])?,
        is_else,
    })
}

fn decode_when_body(form: &Form) -> Result<WhenBody, ArtifactError> {
    let list = expect_nonempty_list(form, "when body")?;
    expect_len(list, 2, "when body")?;
    match atom(&list[0])? {
        "when-expr" => Ok(WhenBody::Expr(decode_expr(&list[1])?)),
        "when-block" => Ok(WhenBody::Block(decode_block(&list[1])?)),
        tag => invalid(format!("unknown when body tag '{tag}'")),
    }
}

fn decode_optional_expr(form: &Form) -> Result<Option<Expr>, ArtifactError> {
    let list = expect_nonempty_list(form, "optional expression")?;
    match atom(&list[0])? {
        "none" => {
            expect_len(list, 1, "none")?;
            Ok(None)
        }
        "some" => {
            expect_len(list, 2, "some")?;
            Ok(Some(decode_expr(&list[1])?))
        }
        tag => invalid(format!("unknown optional expression tag '{tag}'")),
    }
}

fn decode_optional_string(form: &Form) -> Result<Option<String>, ArtifactError> {
    let list = expect_nonempty_list(form, "optional string")?;
    match atom(&list[0])? {
        "none" => {
            expect_len(list, 1, "none")?;
            Ok(None)
        }
        "some" => {
            expect_len(list, 2, "some")?;
            Ok(Some(string(&list[1])?.to_owned()))
        }
        tag => invalid(format!("unknown optional string tag '{tag}'")),
    }
}

fn decode_unary(value: &str) -> Result<UnaryOp, ArtifactError> {
    match value {
        "negate" => Ok(UnaryOp::Negate),
        "not" => Ok(UnaryOp::Not),
        other => invalid(format!("unknown unary operator '{other}'")),
    }
}

fn decode_binary(value: &str) -> Result<BinaryOp, ArtifactError> {
    use BinaryOp::*;
    Ok(match value {
        "elvis" => Elvis,
        "or" => Or,
        "and" => And,
        "equal" => Equal,
        "not-equal" => NotEqual,
        "less" => Less,
        "less-equal" => LessEqual,
        "greater" => Greater,
        "greater-equal" => GreaterEqual,
        "add" => Add,
        "subtract" => Subtract,
        "multiply" => Multiply,
        "divide" => Divide,
        "remainder" => Remainder,
        other => return invalid(format!("unknown binary operator '{other}'")),
    })
}

fn expect_list<'a>(form: &'a Form, context: &str) -> Result<&'a [Form], ArtifactError> {
    match form {
        Form::List(values) => Ok(values),
        _ => invalid(format!("expected {context} list")),
    }
}

fn expect_nonempty_list<'a>(form: &'a Form, context: &str) -> Result<&'a [Form], ArtifactError> {
    let list = expect_list(form, context)?;
    if list.is_empty() {
        invalid(format!("empty {context} list"))
    } else {
        Ok(list)
    }
}

fn expect_tag(list: &[Form], expected: &str) -> Result<(), ArtifactError> {
    if list.first().is_some_and(|form| atom(form) == Ok(expected)) {
        Ok(())
    } else {
        invalid(format!("expected '{expected}' artifact tag"))
    }
}

fn expect_len(list: &[Form], expected: usize, context: &str) -> Result<(), ArtifactError> {
    if list.len() == expected {
        Ok(())
    } else {
        invalid(format!(
            "{context} expects {} fields, got {}",
            expected.saturating_sub(1),
            list.len().saturating_sub(1)
        ))
    }
}

fn atom(form: &Form) -> Result<&str, ArtifactError> {
    match form {
        Form::Atom(value) => Ok(value),
        _ => invalid("expected artifact atom"),
    }
}

fn string(form: &Form) -> Result<&str, ArtifactError> {
    match form {
        Form::String(value) => Ok(value),
        _ => invalid("expected artifact string"),
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, ArtifactError> {
    Err(ArtifactError::new(message, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_minimal_program() {
        let artifact =
            "VELRA-AST-1\n(program (assign (ident \"x\") (int 42)) (expr (ident \"x\")))";
        let program = decode(artifact).unwrap();
        assert_eq!(program.statements.len(), 2);
    }

    #[test]
    fn decodes_escaped_strings() {
        let artifact = "VELRA-AST-1\n(program (expr (string \"a\\n\\\"b\")))";
        let program = decode(artifact).unwrap();
        assert!(matches!(
            &program.statements[0],
            Stmt::Expr(Expr::String(value)) if value == "a\n\"b"
        ));
    }

    #[test]
    fn reports_unicode_string_escape_offsets_in_bytes() {
        let body = "(\"é\\x\")";
        let mut parser = FormParser::new(body, HEADER.len());
        let error = parser.parse_form().unwrap_err();
        assert_eq!(error.offset, HEADER.len() + 5);
    }
}
