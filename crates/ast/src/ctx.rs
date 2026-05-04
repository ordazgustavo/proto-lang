use crate::{interner::Interner, item::ItemArena};

pub struct AstCtx {
    // pub exprs: ExprArena,
    // pub stmts: StmtArena,
    // pub types: TypeArena,
    pub items: ItemArena,
    pub strings: Interner,
    // pub files: Vec<SourceFile>,
    // /// Top-level items per file, in source order.
    // pub items_by_file: Vec<Vec<ItemId>>,
}

impl AstCtx {
    pub fn new() -> Self {
        Self {
            items: ItemArena::new(),
            strings: Interner::with_capacity(256),
        }
    }
}
