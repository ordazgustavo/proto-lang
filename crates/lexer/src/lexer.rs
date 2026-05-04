use crate::token::{LexErrorKind, Span, Token, TokenKind};

pub struct Lexer<'a> {
    bytes: &'a [u8],
    _file: u16,
    pos: usize,
    emitted_eof: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self::with_file(source, 0)
    }

    pub fn with_file(source: &'a str, file: u16) -> Self {
        Self {
            bytes: source.as_bytes(),
            _file: file,
            pos: 0,
            emitted_eof: false,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn match_next(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
                self.pos += 1;
            }

            if self.peek() == Some(b'/') && self.peek_next() == Some(b'/') {
                self.pos += 2;
                while let Some(b) = self.peek() {
                    self.pos += 1;
                    if b == b'\n' {
                        break;
                    }
                }
                continue;
            }

            break;
        }
    }

    fn is_ident_start(byte: u8) -> bool {
        byte.is_ascii_alphabetic() || byte == b'_'
    }

    fn is_ident_continue(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    fn lex_identifier_or_keyword(&mut self, start: usize) -> Token {
        while let Some(b) = self.peek() {
            if Self::is_ident_continue(b) {
                self.pos += 1;
            } else {
                break;
            }
        }

        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or_default();
        let kind = match text {
            "const" => TokenKind::KwConst,
            "else" => TokenKind::KwElse,
            "enum" => TokenKind::KwEnum,
            "extend" => TokenKind::KwExtend,
            "fn" => TokenKind::KwFn,
            "for" => TokenKind::KwFor,
            "if" => TokenKind::KwIf,
            "import" => TokenKind::KwImport,
            "in" => TokenKind::KwIn,
            "let" => TokenKind::KwLet,
            "self" => TokenKind::KwSelf,
            "struct" => TokenKind::KwStruct,
            "true" => TokenKind::KwTrue,
            "false" => TokenKind::KwFalse,
            "var" => TokenKind::KwVar,
            _ => TokenKind::Ident,
        };

        Token {
            kind,
            span: Span {
                start,
                end: self.pos,
            },
        }
    }

    fn lex_number(&mut self, start: usize) -> Token {
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.peek() == Some(b'.') {
            let dot_pos = self.pos;
            self.pos += 1;
            let frac_start = self.pos;
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }

            if self.pos == frac_start {
                return Token {
                    kind: TokenKind::Error(LexErrorKind::MalformedFloat),
                    span: Span {
                        start,
                        end: dot_pos + 1,
                    },
                };
            }

            return Token {
                kind: TokenKind::Float,
                span: Span {
                    start,
                    end: self.pos,
                },
            };
        }

        Token {
            kind: TokenKind::Integer,
            span: Span {
                start,
                end: self.pos,
            },
        }
    }

    fn lex_string(&mut self, start: usize) -> Token {
        let mut invalid_escape = false;

        while let Some(ch) = self.bump() {
            match ch {
                b'"' => {
                    let kind = if invalid_escape {
                        TokenKind::Error(LexErrorKind::InvalidEscape)
                    } else {
                        TokenKind::Str
                    };
                    return Token {
                        kind,
                        span: Span {
                            start,
                            end: self.pos,
                        },
                    };
                }
                b'\\' => {
                    let Some(next) = self.bump() else {
                        return Token {
                            kind: TokenKind::Error(LexErrorKind::UnterminatedString),
                            span: Span {
                                start,
                                end: self.pos,
                            },
                        };
                    };
                    if !matches!(next, b'n' | b'r' | b't' | b'\\' | b'"' | b'\'') {
                        invalid_escape = true;
                    }
                }
                b'\n' | b'\r' => {
                    return Token {
                        kind: TokenKind::Error(LexErrorKind::UnterminatedString),
                        span: Span {
                            start,
                            end: self.pos,
                        },
                    };
                }
                _ => {}
            }
        }

        Token {
            kind: TokenKind::Error(LexErrorKind::UnterminatedString),
            span: Span {
                start,
                end: self.pos,
            },
        }
    }

    fn lex_char(&mut self, start: usize) -> Token {
        let Some(ch) = self.bump() else {
            return Token {
                kind: TokenKind::Error(LexErrorKind::UnterminatedChar),
                span: Span {
                    start,
                    end: self.pos,
                },
            };
        };

        match ch {
            b'\\' => {
                let Some(escaped) = self.bump() else {
                    return Token {
                        kind: TokenKind::Error(LexErrorKind::UnterminatedChar),
                        span: Span {
                            start,
                            end: self.pos,
                        },
                    };
                };
                if !matches!(escaped, b'n' | b'r' | b't' | b'\\' | b'"' | b'\'') {
                    return Token {
                        kind: TokenKind::Error(LexErrorKind::InvalidEscape),
                        span: Span {
                            start,
                            end: self.pos,
                        },
                    };
                }
            }
            b'\n' | b'\r' | b'\'' => {
                return Token {
                    kind: TokenKind::Error(LexErrorKind::InvalidCharLiteral),
                    span: Span {
                        start,
                        end: self.pos,
                    },
                };
            }
            _ => {}
        }

        if self.match_next(b'\'') {
            Token {
                kind: TokenKind::Char,
                span: Span {
                    start,
                    end: self.pos,
                },
            }
        } else {
            Token {
                kind: TokenKind::Error(LexErrorKind::UnterminatedChar),
                span: Span {
                    start,
                    end: self.pos,
                },
            }
        }
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.emitted_eof {
            return None;
        }

        self.skip_trivia();

        let start = self.pos;
        let Some(ch) = self.bump() else {
            self.emitted_eof = true;
            return Some(Token {
                kind: TokenKind::Eof,
                span: Span { start, end: start },
            });
        };

        let token = match ch {
            b if Self::is_ident_start(b) => {
                self.pos -= 1;
                self.lex_identifier_or_keyword(start)
            }
            b if b.is_ascii_digit() => {
                self.pos -= 1;
                self.lex_number(start)
            }
            b'"' => self.lex_string(start),
            b'\'' => self.lex_char(start),

            b'(' => Token {
                kind: TokenKind::LParen,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b')' => Token {
                kind: TokenKind::RParen,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'{' => Token {
                kind: TokenKind::LBrace,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'}' => Token {
                kind: TokenKind::RBrace,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'[' => Token {
                kind: TokenKind::LBracket,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b']' => Token {
                kind: TokenKind::RBracket,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b',' => Token {
                kind: TokenKind::Comma,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b':' => {
                let kind = if self.match_next(b':') {
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                };
                Token {
                    kind,
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }
            }
            b'.' => Token {
                kind: TokenKind::Dot,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'+' => Token {
                kind: TokenKind::Plus,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'-' => {
                let kind = if self.match_next(b'>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                };
                Token {
                    kind,
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }
            }
            b'*' => Token {
                kind: TokenKind::Star,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'/' => Token {
                kind: TokenKind::Slash,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'%' => Token {
                kind: TokenKind::Percent,
                span: Span {
                    start,
                    end: self.pos,
                },
            },
            b'=' => {
                let kind = if self.match_next(b'=') {
                    TokenKind::EqEq
                } else {
                    TokenKind::Assign
                };
                Token {
                    kind,
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }
            }
            b'!' => {
                let kind = if self.match_next(b'=') {
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                };
                Token {
                    kind,
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }
            }
            b'<' => {
                let kind = if self.match_next(b'=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                };
                Token {
                    kind,
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }
            }
            b'>' => {
                let kind = if self.match_next(b'=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                };
                Token {
                    kind,
                    span: Span {
                        start,
                        end: self.pos,
                    },
                }
            }
            _ => Token {
                kind: TokenKind::Error(LexErrorKind::UnknownToken),
                span: Span {
                    start,
                    end: self.pos,
                },
            },
        };

        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::{LexErrorKind, Lexer, TokenKind};

    fn kinds(source: &str) -> Vec<TokenKind> {
        Lexer::new(source).map(|token| token.kind).collect()
    }

    #[test]
    fn lexes_keywords_and_identifiers() {
        let got = kinds("let value = self");
        assert_eq!(
            got,
            vec![
                TokenKind::KwLet,
                TokenKind::Ident,
                TokenKind::Assign,
                TokenKind::KwSelf,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lexes_multi_char_operators() {
        let got = kinds(":: -> <= >= == !=");
        assert_eq!(
            got,
            vec![
                TokenKind::ColonColon,
                TokenKind::Arrow,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lexes_literals_and_reports_errors() {
        let got = kinds(r#"123 0.5 'a' "ok" "\q""#);
        assert_eq!(
            got,
            vec![
                TokenKind::Integer,
                TokenKind::Float,
                TokenKind::Char,
                TokenKind::Str,
                TokenKind::Error(LexErrorKind::InvalidEscape),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn lexes_kitchen_synk_prefix() {
        let sample = include_str!("../../../assets/kitchen_synk.pr");
        let first = Lexer::new(sample)
            .take(7)
            .map(|token| token.kind)
            .collect::<Vec<_>>();

        assert_eq!(
            first,
            vec![
                TokenKind::KwImport,
                TokenKind::Ident,
                TokenKind::KwImport,
                TokenKind::Ident,
                TokenKind::ColonColon,
                TokenKind::Ident,
                TokenKind::KwStruct
            ]
        );
    }

    #[test]
    fn tracks_spans() {
        let tokens = Lexer::new("let x").take(2).collect::<Vec<_>>();
        assert_eq!(tokens[0].span.start, 0);
        assert_eq!(tokens[0].span.end, 3);
        assert_eq!(tokens[1].span.start, 4);
        assert_eq!(tokens[1].span.end, 5);
    }
}
