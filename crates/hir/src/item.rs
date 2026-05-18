use ast::{
    common::{Ident, ModPath},
    span::{FileId, Span},
};

use crate::ids::{
    BodyId, DefId, ExprId, FieldId, GenericParamId, ImportId, ItemId, LocalId, ParamId, ScopeId,
    TypeId, VariantId,
};

#[derive(Debug)]
pub struct Module {
    pub file: FileId,
    pub imports: Vec<ImportId>,
    pub items: Vec<ItemId>,
    pub value_items: Vec<ItemId>,
    pub type_items: Vec<ItemId>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: ModPath,
    pub span: Span,
}

#[derive(Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub def: Option<DefId>,
    pub span: Span,
}

#[derive(Debug)]
pub enum ItemKind {
    Const(Const),
    Function(Function),
    Method(Method),
    Struct(Struct),
    Enum(Enum),
    Extend(Extend),
}

#[derive(Debug)]
pub struct Const {
    pub name: Ident,
    pub ty: TypeId,
    pub value: ExprId,
}

#[derive(Debug)]
pub struct Function {
    pub name: Ident,
    pub generic_params: Vec<GenericParamId>,
    pub params: Vec<ParamId>,
    pub return_type: Option<TypeId>,
    pub body: BodyId,
}

#[derive(Debug)]
pub struct Method {
    pub target: Ident,
    pub name: Ident,
    pub generic_params: Vec<GenericParamId>,
    pub params: Vec<ParamId>,
    pub return_type: Option<TypeId>,
    pub body: BodyId,
}

#[derive(Debug)]
pub struct Struct {
    pub name: Ident,
    pub generic_params: Vec<GenericParamId>,
    pub fields: Vec<FieldId>,
}

#[derive(Debug)]
pub struct Enum {
    pub name: Ident,
    pub generic_params: Vec<GenericParamId>,
    pub variants: Vec<VariantId>,
}

#[derive(Debug)]
pub struct Extend {
    pub target: Ident,
    pub generic_params: Vec<GenericParamId>,
    pub methods: Vec<ItemId>,
}

#[derive(Debug)]
pub struct GenericParam {
    pub def: DefId,
    pub name: Ident,
    pub span: Span,
}

#[derive(Debug)]
pub struct Param {
    pub def: DefId,
    pub label: ParamLabel,
    pub name: Ident,
    pub ty: TypeId,
    pub span: Span,
}

#[derive(Debug)]
pub struct Field {
    pub def: DefId,
    pub label: ParamLabel,
    pub name: Ident,
    pub ty: TypeId,
    pub span: Span,
}

#[derive(Debug)]
pub struct Variant {
    pub def: DefId,
    pub name: Ident,
    pub payload: Vec<TypeId>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy)]
pub enum ParamLabel {
    Implicit,
    Explicit(Ident),
    Suppressed,
}

#[derive(Debug)]
pub struct Body {
    pub stmts: Vec<Stmt>,
    pub scope: ScopeId,
    pub span: Span,
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub params: Vec<ParamId>,
    pub locals: Vec<LocalId>,
}

#[derive(Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum StmtKind {
    Expr(ExprId),
    Assignment(Assignment),
    If(IfStmt),
    ForIn(ForInStmt),
}

#[derive(Debug)]
pub struct Assignment {
    pub binding: Option<BindingKind>,
    pub local: Option<LocalId>,
    pub target: AssignTarget,
    pub ty: Option<TypeId>,
    pub value: ExprId,
}

#[derive(Debug, Clone, Copy)]
pub enum BindingKind {
    Let,
    Var,
}

#[derive(Debug)]
pub enum AssignTarget {
    Ident(Ident),
    Field { base: Ident, fields: Vec<Ident> },
}

#[derive(Debug)]
pub struct Local {
    pub def: DefId,
    pub binding: BindingKind,
    pub name: Ident,
    pub ty: Option<TypeId>,
    pub span: Span,
}

#[derive(Debug)]
pub struct IfStmt {
    pub condition: ExprId,
    pub then_body: BodyId,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug)]
pub enum ElseBranch {
    If(Box<IfStmt>),
    Body(BodyId),
}

#[derive(Debug)]
pub struct ForInStmt {
    pub local: LocalId,
    pub iter: ExprId,
    pub body: BodyId,
}
