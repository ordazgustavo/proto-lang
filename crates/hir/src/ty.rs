use ast::{common::ModPath, span::Span};

use crate::ids::TypeId;

#[derive(Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum TypeKind {
    Path {
        path: ModPath,
        generic_args: Vec<TypeId>,
    },
}
