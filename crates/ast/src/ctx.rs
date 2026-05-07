use crate::{
    expr::{Expr, ExprArena, ExprId},
    interner::{Interner, StrId},
    item::{Item, ItemArena, ItemId},
    ty::{Type, TypeArena, TypeId},
};

pub struct AstCtx {
    pub items: ItemArena,
    pub types: TypeArena,
    pub exprs: ExprArena,
    pub strings: Interner,
    // pub stmts: StmtArena,
    // pub files: Vec<SourceFile>,
    // /// Top-level items per file, in source order.
    // pub items_by_file: Vec<Vec<ItemId>>,
}

impl AstCtx {
    pub fn new() -> Self {
        Self {
            items: ItemArena::new(),
            types: TypeArena::new(),
            exprs: ExprArena::new(),
            strings: Interner::with_capacity(256),
        }
    }

    pub fn alloc_item(&mut self, item: Item) -> ItemId {
        self.items.alloc(item)
    }

    pub fn get_item(&self, id: ItemId) -> &Item {
        self.items.get(id)
    }

    pub fn alloc_type(&mut self, ty: Type) -> TypeId {
        self.types.alloc(ty)
    }

    pub fn get_type(&self, id: TypeId) -> &Type {
        self.types.get(id)
    }

    pub fn alloc_expr(&mut self, expr: Expr) -> ExprId {
        self.exprs.alloc(expr)
    }

    pub fn get_expr(&self, id: ExprId) -> &Expr {
        self.exprs.get(id)
    }

    pub fn intern_str(&mut self, value: &str) -> StrId {
        self.strings.intern(value)
    }

    pub fn get_str(&self, id: StrId) -> &str {
        self.strings.lookup(id)
    }
}
