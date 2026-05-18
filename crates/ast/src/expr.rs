use crate::{
    arena::{Arena, Id},
    common::{Ident, ModPath},
    item::Block,
    span::Span,
    ty::TypeId,
};

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Literal(Lit),
    Path(ModPath),
    ImplicitMember(Ident),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Call(CallExpr),
    Field(FieldExpr),
    Array(ArrayExpr),
    If(IfExpr),
}

#[derive(Debug)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: ExprId,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug)]
pub struct BinaryExpr {
    pub op: BinaryOp,
    pub lhs: ExprId,
    pub rhs: ExprId,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Eq,
    NotEq,
}

#[derive(Debug)]
pub struct CallExpr {
    pub callee: ExprId,
    pub generic_args: Vec<TypeId>,
    pub args: Vec<Arg>,
}

#[derive(Debug)]
pub struct Arg {
    pub label: Option<Ident>,
    pub value: ExprId,
}

#[derive(Debug)]
pub struct FieldExpr {
    pub base: ExprId,
    pub field: Ident,
}

#[derive(Debug)]
pub struct ArrayExpr {
    pub elements: Vec<ExprId>,
}

#[derive(Debug)]
pub struct IfExpr {
    pub condition: ExprId,
    pub then_block: Block,
    pub else_branch: IfElseBranch,
}

#[derive(Debug)]
pub enum IfElseBranch {
    If(ExprId),
    Block(Block),
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
