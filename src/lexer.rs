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
    While,
    Break,
    Continue,
    In,
    When,
    Return,
    Require,
    Enum,
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
    Range,
    RangeInclusive,
    Ellipsis,
    PipeForward,
    Pipe,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
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
    Hash,
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
    cursor: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, cursor: 0 }
    }

    fn lex(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.current() {
            match ch {
                ' ' | '\t' | '\r' => self.skip_horizontal_whitespace(),
                '\n' => {
                    let start = self.cursor;
                    self.cursor += 1;
                    tokens.push(Token {
                        kind: TokenKind::Newline,
                        span: Span::new(start, self.cursor),
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

    fn char_at(&self, byte: usize) -> Option<char> {
        let first = *self.source.as_bytes().get(byte)?;
        if first.is_ascii() {
            Some(first as char)
        } else {
            self.source.get(byte..)?.chars().next()
        }
    }

    fn current(&self) -> Option<char> {
        self.char_at(self.cursor)
    }

    fn peek_char(&self) -> Option<char> {
        let current = self.current()?;
        self.char_at(self.cursor + current.len_utf8())
    }

    fn skip_horizontal_whitespace(&mut self) {
        let bytes = self.source.as_bytes();
        while matches!(bytes.get(self.cursor), Some(b' ' | b'\t' | b'\r')) {
            self.cursor += 1;
        }
    }

    fn skip_comment(&mut self) {
        let bytes = self.source.as_bytes();
        while bytes.get(self.cursor).is_some_and(|byte| *byte != b'\n') {
            self.cursor += 1;
        }
    }

    fn number(&mut self) -> Result<Token, LexError> {
        let start = self.cursor;
        let bytes = self.source.as_bytes();

        while bytes
            .get(self.cursor)
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            self.cursor += 1;
        }

        let is_float = bytes.get(self.cursor) == Some(&b'.')
            && bytes
                .get(self.cursor + 1)
                .is_some_and(|byte| byte.is_ascii_digit());

        if is_float {
            self.cursor += 1;
            while bytes
                .get(self.cursor)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                self.cursor += 1;
            }
        }

        let end = self.cursor;
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

        Ok(Token { kind, span })
    }

    fn string(&mut self) -> Result<Token, LexError> {
        let start = self.cursor;
        self.cursor += 1;
        let mut value = String::new();

        while let Some(ch) = self.current() {
            let byte = self.cursor;
            match ch {
                '"' => {
                    self.cursor += 1;
                    return Ok(Token {
                        kind: TokenKind::String(value),
                        span: Span::new(start, self.cursor),
                    });
                }
                '\n' => {
                    return Err(LexError {
                        message: "unterminated string literal".into(),
                        span: Span::new(start, byte),
                    });
                }
                '\\' => {
                    self.cursor += 1;
                    let Some(escape) = self.current() else {
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
                    self.cursor += escape.len_utf8();
                }
                _ => {
                    value.push(ch);
                    self.cursor += ch.len_utf8();
                }
            }
        }

        Err(LexError {
            message: "unterminated string literal".into(),
            span: Span::new(start, self.source.len()),
        })
    }

    fn identifier(&mut self) -> Token {
        let start = self.cursor;
        let first = self.current().expect("identifier starts at current character");
        self.cursor += first.len_utf8();
        while let Some(ch) = self.current() {
            if !is_ident_continue(ch) {
                break;
            }
            self.cursor += ch.len_utf8();
        }

        let end = self.cursor;
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
        let start = self.cursor;
        let rest = &self.source.as_bytes()[start..];

        let (kind, width) = match rest {
            [b'.', b'.', b'=', ..] => (TokenKind::RangeInclusive, 3),
            [b'.', b'.', b'.', ..] => (TokenKind::Ellipsis, 3),
            [b'=', b'=', ..] => (TokenKind::Eq, 2),
            [b'!', b'=', ..] => (TokenKind::Neq, 2),
            [b'<', b'=', ..] => (TokenKind::Lte, 2),
            [b'>', b'=', ..] => (TokenKind::Gte, 2),
            [b'&', b'&', ..] => (TokenKind::And, 2),
            [b'|', b'|', ..] => (TokenKind::Or, 2),
            [b'|', b'>', ..] => (TokenKind::PipeForward, 2),
            [b'.', b'.', ..] => (TokenKind::Range, 2),
            [b'?', b'.', ..] => (TokenKind::SafeDot, 2),
            [b'?', b':', ..] => (TokenKind::Elvis, 2),
            [b'=', b'>', ..] => (TokenKind::FatArrow, 2),
            [b'-', b'>', ..] => (TokenKind::Arrow, 2),
            [b'+', b'=', ..] => (TokenKind::PlusAssign, 2),
            [b'-', b'=', ..] => (TokenKind::MinusAssign, 2),
            [b'*', b'=', ..] => (TokenKind::StarAssign, 2),
            [b'/', b'=', ..] => (TokenKind::SlashAssign, 2),
            [b'%', b'=', ..] => (TokenKind::PercentAssign, 2),
            [b'=', ..] => (TokenKind::Assign, 1),
            [b'+', ..] => (TokenKind::Plus, 1),
            [b'-', ..] => (TokenKind::Minus, 1),
            [b'*', ..] => (TokenKind::Star, 1),
            [b'/', ..] => (TokenKind::Slash, 1),
            [b'%', ..] => (TokenKind::Percent, 1),
            [b'<', ..] => (TokenKind::Lt, 1),
            [b'>', ..] => (TokenKind::Gt, 1),
            [b'!', ..] => (TokenKind::Bang, 1),
            [b'.', ..] => (TokenKind::Dot, 1),
            [b',', ..] => (TokenKind::Comma, 1),
            [b':', ..] => (TokenKind::Colon, 1),
            [b';', ..] => (TokenKind::Semi, 1),
            [b'?', ..] => (TokenKind::Question, 1),
            [b'#', ..] => (TokenKind::Hash, 1),
            [b'|', ..] => (TokenKind::Pipe, 1),
            [b'(', ..] => (TokenKind::LParen, 1),
            [b')', ..] => (TokenKind::RParen, 1),
            [b'{', ..] => (TokenKind::LBrace, 1),
            [b'}', ..] => (TokenKind::RBrace, 1),
            [b'[', ..] => (TokenKind::LBracket, 1),
            [b']', ..] => (TokenKind::RBracket, 1),
            _ => {
                let ch = self.current().expect("symbol starts at current character");
                return Err(LexError {
                    message: format!("unexpected character '{ch}'"),
                    span: Span::new(start, start + ch.len_utf8()),
                });
            }
        };

        self.cursor += width;
        Ok(Token {
            kind,
            span: Span::new(start, self.cursor),
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
        "while" => TokenKind::While,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "in" => TokenKind::In,
        "when" => TokenKind::When,
        "return" => TokenKind::Return,
        "require" => TokenKind::Require,
        "enum" => TokenKind::Enum,
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

    #[test]
    fn tracks_byte_spans_for_unicode_without_preindexing() {
        let tokens = lex("Δ = \"é\"").unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 2));
        assert_eq!(tokens[1].span, Span::new(3, 4));
        assert_eq!(tokens[2].span, Span::new(5, 9));
        assert_eq!(tokens[3].span, Span::new(9, 9));
    }

    #[test]
    fn reports_full_utf8_width_for_unexpected_characters() {
        let error = lex("🙂").unwrap_err();
        assert_eq!(error.span, Span::new(0, 4));
    }
}
