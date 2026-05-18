#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImportId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariantId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericParamId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub(crate) usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub(crate) usize);
