use crate::{
    def::Def,
    expr::Expr,
    ids::*,
    item::{
        Body, Field, GenericParam, Import, Item, ItemKind, Local, Module, Param, Scope, Variant,
    },
    ty::Type,
};

#[derive(Debug)]
pub struct HirCtx {
    modules: Vec<Module>,
    imports: Vec<Import>,
    items: Vec<Item>,
    bodies: Vec<Body>,
    exprs: Vec<Expr>,
    types: Vec<Type>,
    locals: Vec<Local>,
    params: Vec<Param>,
    fields: Vec<Field>,
    variants: Vec<Variant>,
    generic_params: Vec<GenericParam>,
    scopes: Vec<Scope>,
    defs: Vec<Def>,
}

impl HirCtx {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            imports: Vec::new(),
            items: Vec::new(),
            bodies: Vec::new(),
            exprs: Vec::new(),
            types: Vec::new(),
            locals: Vec::new(),
            params: Vec::new(),
            fields: Vec::new(),
            variants: Vec::new(),
            generic_params: Vec::new(),
            scopes: Vec::new(),
            defs: Vec::new(),
        }
    }

    pub fn module(&self, id: ModuleId) -> &Module {
        &self.modules[id.0]
    }
    pub fn import(&self, id: ImportId) -> &Import {
        &self.imports[id.0]
    }
    pub fn item(&self, id: ItemId) -> &Item {
        &self.items[id.0]
    }
    pub fn body(&self, id: BodyId) -> &Body {
        &self.bodies[id.0]
    }
    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id.0]
    }
    pub fn ty(&self, id: TypeId) -> &Type {
        &self.types[id.0]
    }
    pub fn param(&self, id: ParamId) -> &Param {
        &self.params[id.0]
    }
    pub fn field(&self, id: FieldId) -> &Field {
        &self.fields[id.0]
    }
    pub fn variant(&self, id: VariantId) -> &Variant {
        &self.variants[id.0]
    }
    pub fn local(&self, id: LocalId) -> &Local {
        &self.locals[id.0]
    }
    pub fn generic_param(&self, id: GenericParamId) -> &GenericParam {
        &self.generic_params[id.0]
    }
    pub fn scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0]
    }
    pub fn def(&self, id: DefId) -> &Def {
        &self.defs[id.0]
    }
    pub fn module_value_items(&self, id: ModuleId) -> &[ItemId] {
        &self.module(id).value_items
    }
    pub fn module_type_items(&self, id: ModuleId) -> &[ItemId] {
        &self.module(id).type_items
    }
    pub fn scope_parent(&self, id: ScopeId) -> Option<ScopeId> {
        self.scope(id).parent
    }
    pub fn scope_params(&self, id: ScopeId) -> &[ParamId] {
        &self.scope(id).params
    }
    pub fn scope_locals(&self, id: ScopeId) -> &[LocalId] {
        &self.scope(id).locals
    }
    pub fn item_def(&self, id: ItemId) -> Option<DefId> {
        self.item(id).def
    }
    pub fn item_name(&self, id: ItemId) -> Option<ast::common::Ident> {
        match &self.item(id).kind {
            ItemKind::Const(def) => Some(def.name),
            ItemKind::Function(def) => Some(def.name),
            ItemKind::Method(def) => Some(def.name),
            ItemKind::Struct(def) => Some(def.name),
            ItemKind::Enum(def) => Some(def.name),
            ItemKind::Extend(_) => None,
        }
    }
    pub fn item_path(&self, id: ItemId) -> Vec<ast::common::Ident> {
        match &self.item(id).kind {
            ItemKind::Method(def) => vec![def.target, def.name],
            _ => self.item_name(id).into_iter().collect(),
        }
    }
    pub fn struct_fields(&self, id: ItemId) -> &[FieldId] {
        match &self.item(id).kind {
            ItemKind::Struct(def) => &def.fields,
            _ => &[],
        }
    }
    pub fn enum_variants(&self, id: ItemId) -> &[VariantId] {
        match &self.item(id).kind {
            ItemKind::Enum(def) => &def.variants,
            _ => &[],
        }
    }
    pub fn extend_methods(&self, id: ItemId) -> &[ItemId] {
        match &self.item(id).kind {
            ItemKind::Extend(def) => &def.methods,
            _ => &[],
        }
    }

    pub(crate) fn alloc_module(&mut self, val: Module) -> ModuleId {
        let id = ModuleId(self.modules.len());
        self.modules.push(val);
        id
    }
    pub(crate) fn alloc_import(&mut self, val: Import) -> ImportId {
        let id = ImportId(self.imports.len());
        self.imports.push(val);
        id
    }
    pub(crate) fn alloc_item(&mut self, val: Item) -> ItemId {
        let id = ItemId(self.items.len());
        self.items.push(val);
        id
    }
    pub(crate) fn item_mut(&mut self, id: ItemId) -> &mut Item {
        &mut self.items[id.0]
    }
    pub(crate) fn alloc_body(&mut self, val: Body) -> BodyId {
        let id = BodyId(self.bodies.len());
        self.bodies.push(val);
        id
    }
    pub(crate) fn alloc_expr(&mut self, val: Expr) -> ExprId {
        let id = ExprId(self.exprs.len());
        self.exprs.push(val);
        id
    }
    pub(crate) fn alloc_type(&mut self, val: Type) -> TypeId {
        let id = TypeId(self.types.len());
        self.types.push(val);
        id
    }
    pub(crate) fn alloc_local(&mut self, val: Local) -> LocalId {
        let id = LocalId(self.locals.len());
        self.locals.push(val);
        id
    }
    pub(crate) fn alloc_param(&mut self, val: Param) -> ParamId {
        let id = ParamId(self.params.len());
        self.params.push(val);
        id
    }
    pub(crate) fn alloc_field(&mut self, val: Field) -> FieldId {
        let id = FieldId(self.fields.len());
        self.fields.push(val);
        id
    }
    pub(crate) fn alloc_variant(&mut self, val: Variant) -> VariantId {
        let id = VariantId(self.variants.len());
        self.variants.push(val);
        id
    }
    pub(crate) fn alloc_generic_param(&mut self, val: GenericParam) -> GenericParamId {
        let id = GenericParamId(self.generic_params.len());
        self.generic_params.push(val);
        id
    }
    pub(crate) fn alloc_scope(&mut self, val: Scope) -> ScopeId {
        let id = ScopeId(self.scopes.len());
        self.scopes.push(val);
        id
    }
    pub(crate) fn alloc_def(&mut self, val: Def) -> DefId {
        let id = DefId(self.defs.len());
        self.defs.push(val);
        id
    }
    pub(crate) fn next_item_id(&self) -> ItemId {
        ItemId(self.items.len())
    }
    pub(crate) fn next_local_id(&self) -> LocalId {
        LocalId(self.locals.len())
    }
    pub(crate) fn next_param_id(&self) -> ParamId {
        ParamId(self.params.len())
    }
    pub(crate) fn next_field_id(&self) -> FieldId {
        FieldId(self.fields.len())
    }
    pub(crate) fn next_variant_id(&self) -> VariantId {
        VariantId(self.variants.len())
    }
    pub(crate) fn next_generic_param_id(&self) -> GenericParamId {
        GenericParamId(self.generic_params.len())
    }
    pub(crate) fn scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id.0]
    }
}

impl Default for HirCtx {
    fn default() -> Self {
        Self::new()
    }
}
