use crate::{
    arena::{Arena, Id},
    common::ModPath,
    span::Span,
};

#[derive(Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub struct ItemId(pub(crate) u32);
pub type ItemArena = Arena<Item>;
pub type ItemId = Id<Item>;

#[derive(Debug)]
pub enum ItemKind {
    Import(ImportDef),
    // Const,
    // Function,
    // Struct,
    // Enum,
    // Extend,
}

#[derive(Debug)]
pub struct ImportDef {
    pub path: ModPath,
}
