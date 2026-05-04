use ast::span::Span;

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexErrorKind {
    UnknownToken,
    UnterminatedString,
    UnterminatedChar,
    InvalidEscape,
    InvalidCharLiteral,
    MalformedFloat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Eof,
    Error(LexErrorKind),

    // Identifiers and literals
    Ident,
    Integer,
    Float,
    Char,
    Str,

    // Keywords
    KwConst,
    KwElse,
    KwEnum,
    KwExtend,
    KwFn,
    KwFor,
    KwIf,
    KwImport,
    KwIn,
    KwLet,
    KwSelf,
    KwStruct,
    KwTrue,
    KwFalse,
    KwVar,

    // Delimiters and punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    ColonColon,
    Dot,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Bang,
    Arrow,
}
