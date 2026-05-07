use crate::{
    ctx::AstCtx,
    item::{ConstDef, ImportDef, ItemId, ItemKind},
};

pub trait AstVisitor {
    fn visit_item(&mut self, item_id: ItemId, ctx: &AstCtx) {
        walk_item(self, item_id, ctx);
    }

    fn visit_import(&mut self, _item_id: ItemId, _import: &ImportDef, _ctx: &AstCtx) {}

    fn visit_const(&mut self, _item_id: ItemId, _const_def: &ConstDef, _ctx: &AstCtx) {}
}

pub fn walk_program<V: AstVisitor + ?Sized>(visitor: &mut V, items: &[ItemId], ctx: &AstCtx) {
    for &item_id in items {
        visitor.visit_item(item_id, ctx);
    }
}

pub fn walk_item<V: AstVisitor + ?Sized>(visitor: &mut V, item_id: ItemId, ctx: &AstCtx) {
    let item = ctx.items.get(item_id);
    match &item.kind {
        ItemKind::Import(import) => visitor.visit_import(item_id, import, ctx),
        ItemKind::Const(const_def) => visitor.visit_const(item_id, const_def, ctx),
    }
}
