use crate::{
    ctx::AstCtx,
    item::{ConstDef, EnumDef, FunctionDef, ImportDef, ItemId, ItemKind, StructDef},
};

pub trait AstVisitor {
    fn visit_item(&mut self, item_id: ItemId, ctx: &AstCtx) {
        walk_item(self, item_id, ctx);
    }

    fn visit_import(&mut self, _item_id: ItemId, _import: &ImportDef, _ctx: &AstCtx) {}

    fn visit_const(&mut self, _item_id: ItemId, _const_def: &ConstDef, _ctx: &AstCtx) {}

    fn visit_function(&mut self, _item_id: ItemId, _function: &FunctionDef, _ctx: &AstCtx) {}

    fn visit_struct(&mut self, _item_id: ItemId, _struct_def: &StructDef, _ctx: &AstCtx) {}

    fn visit_enum(&mut self, _item_id: ItemId, _enum_def: &EnumDef, _ctx: &AstCtx) {}
}

pub fn walk_program<V: AstVisitor + ?Sized>(visitor: &mut V, items: &[ItemId], ctx: &AstCtx) {
    for &item_id in items {
        visitor.visit_item(item_id, ctx);
    }
}

pub fn walk_item<V: AstVisitor + ?Sized>(visitor: &mut V, item_id: ItemId, ctx: &AstCtx) {
    let item = ctx.get_item(item_id);
    match &item.kind {
        ItemKind::Import(import) => visitor.visit_import(item_id, import, ctx),
        ItemKind::Const(const_def) => visitor.visit_const(item_id, const_def, ctx),
        ItemKind::Function(function) => visitor.visit_function(item_id, function, ctx),
        ItemKind::Struct(struct_def) => visitor.visit_struct(item_id, struct_def, ctx),
        ItemKind::Enum(enum_def) => visitor.visit_enum(item_id, enum_def, ctx),
    }
}
