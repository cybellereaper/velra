use crate::ast::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    DataIdent(String),
    Int(i64),
    Float(f64),
    String(String),
    True,
    False,
    Null,
    Use,
    Pub,
    Var,
    If,
    Else,
    For,
    In,
    When,
    Return,
    Object,
    Extend,
    Shape,
    Async,
    Await,
    Try,
    Throw,
    FatArrow,
    Arrow,
    Eq,
    Neq,
    Lte,
    Gte,
    And,
    Or,
    SafeDot,
    Elvis,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Lt,
    Gt,
    Bang,
    Dot,
    Comma,
    Colon,
    Semi,
    Question,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Newline,
    Eof,
}

impl TokenKind {
    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Newline | Self::Semi)
    }

    pub fn is_identifier(&self) -> bool {
        matches!(self, Self::Ident(_) | Self::DataIdent(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message, self.span.start)
    }
}

impl std::error::Error for LexError {}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex()
}

struct Lexer<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().collect(),
            cursor: 0,
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some((start, ch)) = self.current() {
            match ch {
                ' ' | '\t' | '\r' => self.cursor += 1,
                '\n' => {
                    self.cursor += 1;
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        span: Span::new(start, start + 1),
                    });
                }
                '/' if self.peek_char() == Some('/') => self.skip_comment(),
                '0'..='9' => tokens.push(self.number()?),
                '"' => tokens.push(self.string()?),
                c if is_ident_start(c) => tokens.push(self.identifier()),
                _ => tokens.push(self.symbol()?),
            }
        }

        let end = self.source.len();
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(end, end),
        });
        Ok(tokens)
    }

    fn current(&self) -> Option<(usize, char)> {
        self.chars.get(self.cursor).copied()
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).map(|(_, ch)| *ch)
    }

    fn byte_after(&self, index: usize) -> usize {
        self.chars
            .get(index + 1)
            .map(|(byte, _)| *byte)
            .unwrap_or(self.source.len())
    }

    fn skip_comment(&mut self) {
        while let Some((_, ch)) = self.current() {
            if ch == '\n' {
                break;
            }
            self.cursor += 1;
        }
    }

    fn number(&mut self) -> Result<Token, LexError> {
        let start_cursor = self.cursor;
        let start = self
            .current()
            .expect("number starts at current character")
            .0;

        while self.current().is_some_and(|(_, ch)| ch.is_ascii_digit()) {
            self.cursor += 1;
        }

        let is_float = self.current().is_some_and(|(_, ch)| ch == '.')
            && self
                .chars
                .get(self.cursor + 1)
                .is_some_and(|(_, ch)| ch.is_ascii_digit());

        if is_float {
            self.cursor += 1;
            while self.current().is_some_and(|(_, ch)| ch.is_ascii_digit()) {
                self.cursor += 1;
            }
        }

        let end = if self.cursor == 0 {
            start
        } else {
            self.byte_after(self.cursor - 1)
        };
        let text = &self.source[start..end];
        let span = Span::new(start, end);
        let kind = if is_float {
            TokenKind::Float(text.parse().map_err(|_| LexError {
                message: "invalid floating-point literal".into(),
                span,
            })?)
        } else {
            TokenKind::Int(text.parse().map_err(|_| LexError {
                message: "integer literal is out of range".into(),
                span,
            })?)
        };

        debug_assert!(self.cursor > start_cursor);
        Ok(Token { kind, span })
    }

    fn string(&mut self) -> Result<Token, LexError> {
        let start = self
            .current()
            .expect("string starts at current character")
            .0;
        self.cursor += 1;
        let mut value = String::new();

        while let Some((byte, ch)) = self.current() {
            match ch {
                '"' => {
                    self.cursor += 1;
                    return Ok(Token {
                        kind: TokenKind::String(value),
                        span: Span::new(start, self.byte_after(self.cursor - 1)),
                    });
                }
                '\n' => {
                    return Err(LexError {
                        message: "unterminated string literal".into(),
                        span: Span::new(start, byte),
                    })
                }
                '\\' => {
                    self.cursor += 1;
                    let Some((_, escape)) = self.current() else {
                        return Err(LexError {
                            message: "unterminated string escape".into(),
                            span: Span::new(start, self.source.len()),
                        });
                    };
                    let decoded = match escape {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        '0' => '\0',
                        '$' => '$',
                        other => other,
                    };
                    value.push(decoded);
                    self.cursor += 1;
                }
                _ => {
                    value.push(ch);
                    self.cursor += 1;
                }
            }
        }

        Err(LexError {
            message: "unterminated string literal".into(),
            span: Span::new(start, self.source.len()),
        })
    }

    fn identifier(&mut self) -> Token {
        let start = self
            .current()
            .expect("identifier starts at current character")
            .0;
        let first = self
            .current()
            .expect("identifier starts at current character")
            .1;
        self.cursor += 1;
        while self.current().is_some_and(|(_, ch)| is_ident_continue(ch)) {
            self.cursor += 1;
        }
        let end = self.byte_after(self.cursor - 1);
        let text = &self.source[start..end];
        let kind = keyword(text).unwrap_or_else(|| {
            if first.is_uppercase() {
                TokenKind::DataIdent(text.to_owned())
            } else {
                TokenKind::Ident(text.to_owned())
            }
        });
        Token {
            kind,
            span: Span::new(start, end),
        }
    }

    fn symbol(&mut self) -> Result<Token, LexError> {
        let (start, ch) = self.current().expect("symbol starts at current character");
        let pair = self.peek_char().map(|next| (ch, next));
        let paired = match pair {
            Some(('=', '=')) => Some(TokenKind::Eq),
            Some(('!', '=')) => Some(TokenKind::Neq),
            Some(('<', '=')) => Some(TokenKind::Lte),
            Some(('>', '=')) => Some(TokenKind::Gte),
            Some(('&', '&')) => Some(TokenKind::And),
            Some(('|', '|')) => Some(TokenKind::Or),
            Some(('?', '.')) => Some(TokenKind::SafeDot),
            Some(('?', ':')) => Some(TokenKind::Elvis),
            Some(('=', '>')) => Some(TokenKind::FatArrow),
            Some(('-', '>')) => Some(TokenKind::Arrow),
            _ => None,
        };

        if let Some(kind) = paired {
            self.cursor += 2;
            return Ok(Token {
                kind,
                span: Span::new(start, self.byte_after(self.cursor - 1)),
            });
        }

        let kind = match ch {
            '=' => TokenKind::Assign,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '!' => TokenKind::Bang,
            '.' => TokenKind::Dot,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semi,
            '?' => TokenKind::Question,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            _ => {
                return Err(LexError {
                    message: format!("unexpected character '{ch}'"),
                    span: Span::new(start, self.byte_after(self.cursor)),
                })
            }
        };
        self.cursor += 1;
        Ok(Token {
            kind,
            span: Span::new(start, self.byte_after(self.cursor - 1)),
        })
    }
}

fn keyword(text: &str) -> Option<TokenKind> {
    Some(match text {
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        "use" => TokenKind::Use,
        "pub" => TokenKind::Pub,
        "var" => TokenKind::Var,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "when" => TokenKind::When,
        "return" => TokenKind::Return,
        "object" => TokenKind::Object,
        "extend" => TokenKind::Extend,
        "shape" => TokenKind::Shape,
        "async" => TokenKind::Async,
        "await" => TokenKind::Await,
        "try" => TokenKind::Try,
        "throw" => TokenKind::Throw,
        _ => return None,
    })
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_unicode_identifiers_and_escapes() {
        let tokens = lex("Δelta = \"a\\n\"").unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::DataIdent(name) if name == "Δelta"));
        assert!(matches!(&tokens[2].kind, TokenKind::String(value) if value == "a\n"));
    }

    #[test]
    fn keeps_newlines_but_skips_comments() {
        let tokens = lex("a = 1 // comment\nb = 2").unwrap();
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.kind == TokenKind::Newline)
                .count(),
            1
        );
    }
}
