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
    Function(FunctionDef),
    Struct(StructDef),
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

#[derive(Debug)]
pub struct FunctionDef {
    pub name: Ident,
    pub generic_params: Vec<Ident>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeId>,
    pub body: Block,
}

#[derive(Debug)]
pub struct StructDef {
    pub name: Ident,
    pub generic_params: Vec<Ident>,
    pub fields: Vec<Param>,
}

#[derive(Debug)]
pub struct Param {
    pub label: ParamLabel,
    pub name: Ident,
    pub ty: TypeId,
}

#[derive(Debug)]
pub enum ParamLabel {
    Implicit,
    Explicit(Ident),
    Suppressed,
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum StmtKind {
    Expr(ExprId),
}
