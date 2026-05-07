use crate::{
    arena::{Arena, Id},
    span::Span,
};

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Literal(Lit),
}

#[derive(Debug)]
pub struct Lit {
    pub kind: LitKind,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LitKind {
    Int,
    Float,
    Bool(bool),
    Char,
    Str,
}

pub type ExprArena = Arena<Expr>;
pub type ExprId = Id<Expr>;
