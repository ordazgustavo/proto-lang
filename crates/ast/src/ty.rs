use crate::{
    arena::{Arena, Id},
    common::ModPath,
    span::Span,
};

#[derive(Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum TypeKind {
    Path(ModPath),
}

pub type TypeArena = Arena<Type>;
pub type TypeId = Id<Type>;
