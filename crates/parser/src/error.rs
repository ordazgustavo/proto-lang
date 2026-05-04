use ast::span::Span;
use lexer::token::TokenKind;

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ParseErrorKind {
    /// Expected a specific token, found something else.
    Expected {
        expected: TokenKind,
        found: TokenKind,
    },
    /// Expected one of several tokens.
    ExpectedOneOf {
        expected: Vec<TokenKind>,
        found: TokenKind,
    },
    /// Expected an expression.
    ExpectedExpr { found: TokenKind },
    /// Expected a type.
    ExpectedType { found: TokenKind },
    /// Expected a pattern.
    ExpectedPattern { found: TokenKind },
    /// Expected an item (fn, struct, …).
    ExpectedItem { found: TokenKind },
    /// Reached end of file unexpectedly.
    UnexpectedEof { expected: Option<TokenKind> },
    /// A specific structural error with a message.
    Message(&'static str),
}

impl ParseError {
    pub fn expected(expected: TokenKind, found: TokenKind, span: Span) -> Self {
        Self {
            kind: ParseErrorKind::Expected { expected, found },
            span,
        }
    }

    pub fn expected_expr(found: TokenKind, span: Span) -> Self {
        Self {
            kind: ParseErrorKind::ExpectedExpr { found },
            span,
        }
    }

    pub fn expected_type(found: TokenKind, span: Span) -> Self {
        Self {
            kind: ParseErrorKind::ExpectedType { found },
            span,
        }
    }

    pub fn expected_pattern(found: TokenKind, span: Span) -> Self {
        Self {
            kind: ParseErrorKind::ExpectedPattern { found },
            span,
        }
    }

    pub fn expected_item(found: TokenKind, span: Span) -> Self {
        Self {
            kind: ParseErrorKind::ExpectedItem { found },
            span,
        }
    }

    pub fn unexpected_eof(expected: Option<TokenKind>, span: Span) -> Self {
        Self {
            kind: ParseErrorKind::UnexpectedEof { expected },
            span,
        }
    }

    pub fn message(msg: &'static str, span: Span) -> Self {
        Self {
            kind: ParseErrorKind::Message(msg),
            span,
        }
    }
}

pub type PResult<T> = Result<T, ParseError>;
