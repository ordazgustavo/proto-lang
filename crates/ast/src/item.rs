use crate::{
    arena::{Arena, Id},
    common::{Ident, ModPath},
    expr::ExprId,
    span::Span,
    ty::TypeId,
};

#[derive(Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

pub type ItemArena = Arena<Item>;
pub type ItemId = Id<Item>;

#[derive(Debug)]
pub enum ItemKind {
    Import(ImportDef),
    Const(ConstDef),
    // Function,
    // Struct,
    // Enum,
    // Extend,
}

#[derive(Debug)]
pub struct ImportDef {
    pub path: ModPath,
}

#[derive(Debug)]
pub struct ConstDef {
    pub name: Ident,
    pub ty: TypeId,
    pub value: ExprId,
}
