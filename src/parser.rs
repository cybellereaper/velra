use crate::ast::{
    BinaryOp, Block, DataDecl, ElseBranch, Expr, FunctionBody, FunctionDecl, Param, Program, Stmt,
    UnaryOp, WhenBody, WhenCase,
};
use crate::lexer::{lex, LexError, Token, TokenKind};
use std::fmt;
use std::mem::discriminant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(source: &str) -> Result<Program, crate::Error> {
    let tokens = lex(source).map_err(crate::Error::Lex)?;
    Parser::new(tokens)
        .parse_program()
        .map_err(crate::Error::Parse)
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn from_source(source: &str) -> Result<Self, LexError> {
        Ok(Self::new(lex(source)?))
    }

    pub fn parse_program(mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        self.consume_separators();
        while !self.at(&TokenKind::Eof) {
            statements.push(self.parse_statement()?);
            if self.at(&TokenKind::Eof) {
                break;
            }
            if !self.consume_separators() && !self.at(&TokenKind::RBrace) {
                return Err(self.error("expected a newline or ';' between statements"));
            }
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.eat(&TokenKind::Pub) {
            if self.at(&TokenKind::Pub) {
                return Err(self.error("duplicate 'pub' modifier"));
            }
            let statement = self.parse_statement()?;
            if !matches!(
                statement,
                Stmt::Function(_) | Stmt::Var { .. } | Stmt::Assign { .. } | Stmt::Data(_)
            ) {
                return Err(self.error("'pub' is only valid on declarations"));
            }
            return Ok(Stmt::Pub(Box::new(statement)));
        }

        if self.eat(&TokenKind::Use) {
            return self.parse_use();
        }
        if self.eat(&TokenKind::Var) {
            return self.parse_var();
        }
        if self.eat(&TokenKind::Return) {
            return self.parse_return();
        }
        if self.eat(&TokenKind::For) {
            return self.parse_for();
        }
        if self.at(&TokenKind::If) {
            return Ok(Stmt::Expr(self.parse_if()?));
        }
        if self.at(&TokenKind::When) {
            return Ok(Stmt::Expr(self.parse_when()?));
        }
        if self.looks_like_data_decl() {
            return self.parse_data().map(Stmt::Data);
        }
        if self.looks_like_function_decl() {
            return self.parse_function().map(Stmt::Function);
        }
        if self.looks_like_command_call() {
            return Ok(Stmt::Expr(self.parse_command_call()?));
        }

        let target = self.parse_expression()?;
        if self.eat(&TokenKind::Assign) {
            let value = self.parse_expression()?;
            return Ok(Stmt::Assign { target, value });
        }
        Ok(Stmt::Expr(target))
    }

    fn parse_use(&mut self) -> Result<Stmt, ParseError> {
        let mut path = vec![self.take_identifier()?];
        while self.eat(&TokenKind::Dot) {
            path.push(self.take_identifier()?);
        }
        Ok(Stmt::Use { path })
    }

    fn parse_var(&mut self) -> Result<Stmt, ParseError> {
        let name = self.take_identifier()?;
        let type_name = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type_name()?)
        } else {
            None
        };
        self.expect(&TokenKind::Assign, "expected '=' in variable declaration")?;
        let value = self.parse_expression()?;
        Ok(Stmt::Var {
            name,
            type_name,
            value,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        if self.current().kind.is_separator()
            || self.at(&TokenKind::RBrace)
            || self.at(&TokenKind::Eof)
        {
            Ok(Stmt::Return(None))
        } else {
            Ok(Stmt::Return(Some(self.parse_expression()?)))
        }
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        let name = self.take_identifier()?;
        self.expect(&TokenKind::In, "expected 'in' after loop variable")?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Stmt::For {
            name,
            iterable,
            body,
        })
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, ParseError> {
        let name = self.take_lower_identifier()?;
        let params = self.parse_params()?;
        let body = if self.eat(&TokenKind::FatArrow) {
            FunctionBody::Expr(self.parse_expression()?)
        } else {
            FunctionBody::Block(self.parse_block()?)
        };
        Ok(FunctionDecl { name, params, body })
    }

    fn parse_data(&mut self) -> Result<DataDecl, ParseError> {
        let name = self.take_data_identifier()?;
        let params = self.parse_params()?;
        let mut computed = Vec::new();
        if self.eat(&TokenKind::LBrace) {
            self.consume_separators();
            while !self.at(&TokenKind::RBrace) {
                if self.at(&TokenKind::Eof) {
                    return Err(self.error("unterminated data body"));
                }
                let field = self.take_identifier()?;
                if !(self.eat(&TokenKind::Assign) || self.eat(&TokenKind::FatArrow)) {
                    return Err(self.error("expected '=' or '=>' after computed field name"));
                }
                let value = self.parse_expression()?;
                computed.push((field, value));
                if !self.at(&TokenKind::RBrace) && !self.consume_separators() {
                    return Err(self.error("expected a newline or ';' between data fields"));
                }
            }
            self.expect(&TokenKind::RBrace, "expected '}' after data body")?;
        }
        Ok(DataDecl {
            name,
            params,
            computed,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        self.expect(&TokenKind::LParen, "expected '('")?;
        self.consume_separators();
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let name = self.take_identifier()?;
                let type_name = if self.eat(&TokenKind::Colon) {
                    Some(self.parse_type_name()?)
                } else {
                    None
                };
                params.push(Param { name, type_name });
                self.consume_separators();
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
                self.consume_separators();
            }
        }
        self.expect(&TokenKind::RParen, "expected ')' after parameters")?;
        Ok(params)
    }

    fn parse_type_name(&mut self) -> Result<String, ParseError> {
        let mut name = self.take_identifier()?;
        if self.eat(&TokenKind::Question) {
            name.push('?');
        }
        Ok(name)
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect(&TokenKind::LBrace, "expected '{'")?;
        self.consume_separators();
        let mut statements = Vec::new();
        while !self.at(&TokenKind::RBrace) {
            if self.at(&TokenKind::Eof) {
                return Err(self.error("unterminated block"));
            }
            statements.push(self.parse_statement()?);
            if !self.at(&TokenKind::RBrace) && !self.consume_separators() {
                return Err(self.error("expected a newline or ';' between statements"));
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}'")?;
        Ok(Block { statements })
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        self.expect(&TokenKind::If, "expected 'if'")?;
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;

        let before_newlines = self.cursor;
        while self.eat(&TokenKind::Newline) {}
        let else_branch = if self.eat(&TokenKind::Else) {
            if self.at(&TokenKind::If) {
                Some(ElseBranch::If(Box::new(self.parse_if()?)))
            } else {
                Some(ElseBranch::Block(self.parse_block()?))
            }
        } else {
            self.cursor = before_newlines;
            None
        };

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }

    fn parse_when(&mut self) -> Result<Expr, ParseError> {
        self.expect(&TokenKind::When, "expected 'when'")?;
        let subject = if self.at(&TokenKind::LBrace) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        self.expect(&TokenKind::LBrace, "expected '{' after when subject")?;
        self.consume_separators();

        let mut cases = Vec::new();
        while !self.at(&TokenKind::RBrace) {
            if self.at(&TokenKind::Eof) {
                return Err(self.error("unterminated when expression"));
            }
            cases.push(self.parse_when_case()?);
            if !self.at(&TokenKind::RBrace) && !self.consume_separators() {
                return Err(self.error("expected a newline or ';' between when cases"));
            }
        }
        self.expect(&TokenKind::RBrace, "expected '}' after when cases")?;
        Ok(Expr::When { subject, cases })
    }

    fn parse_when_case(&mut self) -> Result<WhenCase, ParseError> {
        if self.eat(&TokenKind::Else) {
            self.expect(&TokenKind::FatArrow, "expected '=>' after else")?;
            return Ok(WhenCase {
                patterns: Vec::new(),
                guard: None,
                body: self.parse_when_body()?,
                is_else: true,
            });
        }

        let mut patterns = vec![self.parse_expression()?];
        while self.eat(&TokenKind::Comma) {
            self.consume_separators();
            patterns.push(self.parse_expression()?);
        }
        let guard = if self.eat(&TokenKind::If) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&TokenKind::FatArrow, "expected '=>' after when pattern")?;
        Ok(WhenCase {
            patterns,
            guard,
            body: self.parse_when_body()?,
            is_else: false,
        })
    }

    fn parse_when_body(&mut self) -> Result<WhenBody, ParseError> {
        if self.at(&TokenKind::LBrace) {
            Ok(WhenBody::Block(self.parse_block()?))
        } else {
            Ok(WhenBody::Expr(self.parse_expression()?))
        }
    }

    fn parse_command_call(&mut self) -> Result<Expr, ParseError> {
        let callee = Expr::Ident(self.take_identifier()?);
        let mut args = vec![self.parse_expression()?];
        while self.eat(&TokenKind::Comma) {
            args.push(self.parse_expression()?);
        }
        Ok(Expr::Call {
            callee: Box::new(callee),
            args,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_elvis()
    }

    fn parse_elvis(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_or()?;
        while self.eat(&TokenKind::Elvis) {
            let right = self.parse_or()?;
            expr = binary(expr, BinaryOp::Elvis, right);
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;
        while self.eat(&TokenKind::Or) {
            let right = self.parse_and()?;
            expr = binary(expr, BinaryOp::Or, right);
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;
        while self.eat(&TokenKind::And) {
            let right = self.parse_equality()?;
            expr = binary(expr, BinaryOp::And, right);
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = if self.eat(&TokenKind::Eq) {
                Some(BinaryOp::Equal)
            } else if self.eat(&TokenKind::Neq) {
                Some(BinaryOp::NotEqual)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_comparison()?;
            expr = binary(expr, op, right);
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_additive()?;
        loop {
            let op = if self.eat(&TokenKind::Lt) {
                Some(BinaryOp::Less)
            } else if self.eat(&TokenKind::Lte) {
                Some(BinaryOp::LessEqual)
            } else if self.eat(&TokenKind::Gt) {
                Some(BinaryOp::Greater)
            } else if self.eat(&TokenKind::Gte) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_additive()?;
            expr = binary(expr, op, right);
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = if self.eat(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.eat(&TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_multiplicative()?;
            expr = binary(expr, op, right);
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.eat(&TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.eat(&TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else if self.eat(&TokenKind::Percent) {
                Some(BinaryOp::Remainder)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.parse_unary()?;
            expr = binary(expr, op, right);
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.eat(&TokenKind::Minus) {
            return Ok(Expr::Unary {
                op: UnaryOp::Negate,
                expr: Box::new(self.parse_unary()?),
            });
        }
        if self.eat(&TokenKind::Bang) {
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(self.parse_unary()?),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.eat(&TokenKind::LParen) {
                self.consume_separators();
                let mut args = Vec::new();
                if !self.at(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        self.consume_separators();
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                        self.consume_separators();
                    }
                }
                self.expect(&TokenKind::RParen, "expected ')' after arguments")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.eat(&TokenKind::Dot) {
                let name = self.take_identifier()?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    name,
                    safe: false,
                };
            } else if self.eat(&TokenKind::SafeDot) {
                let name = self.take_identifier()?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    name,
                    safe: true,
                };
            } else if self.eat(&TokenKind::LBracket) {
                let index = self.parse_expression()?;
                self.expect(&TokenKind::RBracket, "expected ']' after index")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Null => Ok(Expr::Null),
            TokenKind::True => Ok(Expr::Bool(true)),
            TokenKind::False => Ok(Expr::Bool(false)),
            TokenKind::Int(value) => Ok(Expr::Int(value)),
            TokenKind::Float(value) => Ok(Expr::Float(value)),
            TokenKind::String(value) => Ok(Expr::String(value)),
            TokenKind::Ident(name) | TokenKind::DataIdent(name) => Ok(Expr::Ident(name)),
            TokenKind::LParen => {
                self.consume_separators();
                let expr = self.parse_expression()?;
                self.consume_separators();
                self.expect(&TokenKind::RParen, "expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::LBracket => self.parse_list(),
            TokenKind::If => {
                self.cursor -= 1;
                self.parse_if()
            }
            TokenKind::When => {
                self.cursor -= 1;
                self.parse_when()
            }
            TokenKind::Object
            | TokenKind::Extend
            | TokenKind::Shape
            | TokenKind::Async
            | TokenKind::Await
            | TokenKind::Try
            | TokenKind::Throw => Err(ParseError {
                message: "keyword is reserved for a future language feature".into(),
                offset: token.span.start,
            }),
            _ => Err(ParseError {
                message: "expected expression".into(),
                offset: token.span.start,
            }),
        }
    }

    fn parse_list(&mut self) -> Result<Expr, ParseError> {
        self.consume_separators();
        let mut items = Vec::new();
        if !self.at(&TokenKind::RBracket) {
            loop {
                items.push(self.parse_expression()?);
                self.consume_separators();
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
                self.consume_separators();
            }
        }
        self.expect(&TokenKind::RBracket, "expected ']' after list")?;
        Ok(Expr::List(items))
    }

    fn looks_like_function_decl(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(_))
            && self.after_parameter_clause().is_some_and(|index| {
                matches!(
                    self.tokens[index].kind,
                    TokenKind::FatArrow | TokenKind::LBrace
                )
            })
    }

    fn looks_like_data_decl(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::DataIdent(_)) {
            return false;
        }
        let Some(after) = self.after_parameter_clause() else {
            return false;
        };
        if matches!(self.tokens[after].kind, TokenKind::LBrace) {
            return true;
        }

        let mut depth = 0usize;
        let mut saw_type = false;
        for token in &self.tokens[self.cursor + 1..after] {
            match token.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => depth = depth.saturating_sub(1),
                TokenKind::Colon if depth == 1 => saw_type = true,
                _ => {}
            }
        }
        saw_type
            && (self.tokens[after].kind.is_separator()
                || matches!(self.tokens[after].kind, TokenKind::Eof | TokenKind::RBrace))
    }

    fn after_parameter_clause(&self) -> Option<usize> {
        if !self
            .tokens
            .get(self.cursor + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
        {
            return None;
        }
        let mut depth = 0usize;
        for index in self.cursor + 1..self.tokens.len() {
            match self.tokens[index].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(index + 1);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn looks_like_command_call(&self) -> bool {
        if !self.current().kind.is_identifier() {
            return false;
        }
        let Some(next) = self.tokens.get(self.cursor + 1) else {
            return false;
        };
        self.current().span.end < next.span.start && can_start_command_argument(&next.kind)
    }

    fn take_identifier(&mut self) -> Result<String, ParseError> {
        match self.advance().kind.clone() {
            TokenKind::Ident(name) | TokenKind::DataIdent(name) => Ok(name),
            _ => {
                self.cursor = self.cursor.saturating_sub(1);
                Err(self.error("expected identifier"))
            }
        }
    }

    fn take_lower_identifier(&mut self) -> Result<String, ParseError> {
        match self.advance().kind.clone() {
            TokenKind::Ident(name) => Ok(name),
            _ => {
                self.cursor = self.cursor.saturating_sub(1);
                Err(self.error("function names must begin with a lowercase letter or '_'"))
            }
        }
    }

    fn take_data_identifier(&mut self) -> Result<String, ParseError> {
        match self.advance().kind.clone() {
            TokenKind::DataIdent(name) => Ok(name),
            _ => {
                self.cursor = self.cursor.saturating_sub(1);
                Err(self.error("data type names must begin with an uppercase letter"))
            }
        }
    }

    fn consume_separators(&mut self) -> bool {
        let start = self.cursor;
        while self.current().kind.is_separator() {
            self.cursor += 1;
        }
        self.cursor != start
    }

    fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<(), ParseError> {
        if self.eat(kind) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn at(&self, kind: &TokenKind) -> bool {
        same_variant(&self.current().kind, kind)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor.min(self.tokens.len().saturating_sub(1))]
    }

    fn advance(&mut self) -> &Token {
        let index = self.cursor;
        if !self.at(&TokenKind::Eof) {
            self.cursor += 1;
        }
        &self.tokens[index]
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            offset: self.current().span.start,
        }
    }
}

fn same_variant(left: &TokenKind, right: &TokenKind) -> bool {
    discriminant(left) == discriminant(right)
}

fn binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
    Expr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

fn can_start_command_argument(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Int(_)
            | TokenKind::Float(_)
            | TokenKind::String(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
            | TokenKind::Ident(_)
            | TokenKind::DataIdent(_)
            | TokenKind::LBracket
            | TokenKind::Minus
            | TokenKind::Bang
            | TokenKind::If
            | TokenKind::When
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> Program {
        Parser::from_source(source)
            .unwrap()
            .parse_program()
            .unwrap()
    }

    #[test]
    fn parses_minimal_declarations() {
        let program = parse_ok("name = \"Velra\"\nvar count: Int = 1\nadd(a, b) => a + b");
        assert_eq!(program.statements.len(), 3);
        assert!(matches!(program.statements[1], Stmt::Var { .. }));
        assert!(matches!(program.statements[2], Stmt::Function(_)));
    }

    #[test]
    fn parses_data_when_and_safe_access() {
        let source = r#"
User(name: String) {
    label => name
}
user = User("Ada")
when user?.label {
    "Ada" => true
    else => false
}
"#;
        assert_eq!(parse_ok(source).statements.len(), 3);
    }

    #[test]
    fn parses_command_style_calls() {
        let program = parse_ok("print \"hello\", 42");
        assert!(matches!(
            program.statements[0],
            Stmt::Expr(Expr::Call { .. })
        ));
    }
    #[test]
    fn adjacent_indexing_is_not_a_command_call() {
        let program = parse_ok("value = [1]\nvalue[0]");
        assert!(matches!(
            program.statements[1],
            Stmt::Expr(Expr::Index { .. })
        ));
    }
}
