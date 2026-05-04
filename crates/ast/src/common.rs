use crate::{interner::StrId, span::Span};

#[derive(Debug)]
pub struct Ident {
    pub name: StrId,
    pub span: Span,
}

#[derive(Debug)]
pub struct ModPath(pub Vec<Ident>);
