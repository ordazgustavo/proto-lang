use ast::{common::Ident, span::Span};

use crate::ids::{FieldId, GenericParamId, ItemId, LocalId, ParamId, ScopeId, VariantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefKind {
    Const(ItemId),
    Function(ItemId),
    Method(ItemId),
    Struct(ItemId),
    Enum(ItemId),
    Variant(VariantId),
    Field(FieldId),
    Param(ParamId),
    Local(LocalId),
    Generic(GenericParamId),
}

#[derive(Debug)]
pub struct Def {
    pub kind: DefKind,
    pub name: Ident,
    pub span: Span,
    pub parent_scope: Option<ScopeId>,
    pub parent_item: Option<ItemId>,
}
