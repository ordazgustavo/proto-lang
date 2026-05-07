use crate::{
    expr::ExprArena,
    interner::Interner,
    item::ItemArena,
    ty::TypeArena,
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
}
