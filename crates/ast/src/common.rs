use crate::{interner::StrId, span::Span};

#[derive(Debug, Clone, Copy)]
pub struct Ident {
    pub name: StrId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ModPath(pub Vec<Ident>);
