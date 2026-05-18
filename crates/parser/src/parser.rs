use std::iter::Peekable;

use ast::{
    common::{Ident, ModPath},
    ctx::AstCtx,
    item::{
        AssignTarget, AssignmentStmt, BindingKind, Block, ConstDef, ElseBranch, EnumDef,
        EnumVariant, ExtendDef, ForInStmt, FunctionDef, IfStmt, ImportDef, Item, ItemId, ItemKind,
        Param, ParamLabel, Stmt, StmtKind, StructDef,
    },
    span::{FileId, Span},
    ty::{Type, TypeId, TypeKind},
};
use lexer::{
    Lexer,
    token::{Token, TokenKind},
};

use crate::error::{PResult, ParseError};

pub struct Parser<'a> {
    pub(crate) ctx: &'a mut AstCtx,
    pub(crate) source: &'a str,
    pub(crate) lexer: Peekable<Lexer<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(ctx: &'a mut AstCtx, source: &'a str, file: FileId) -> Self {
        Self {
            ctx,
            source,
            lexer: Lexer::new(source, file).peekable(),
        }
    }

    pub fn parse_program(&mut self) -> PResult<Vec<ItemId>> {
        let mut items = Vec::new();
        while let Some(token) = self.lexer.next() {
            if token.kind == TokenKind::Eof {
                break;
            }
            let item = self.parse_item(token)?;
            items.push(item);
        }
        Ok(items)
    }

    fn parse_item(&mut self, token: Token) -> PResult<ItemId> {
        match token.kind {
            TokenKind::KwImport => self.parse_import_def(token.span),
            TokenKind::KwConst => self.parse_const_def(token.span),
            TokenKind::KwFn => self.parse_function_def(token.span),
            TokenKind::KwStruct => self.parse_struct_def(token.span),
            TokenKind::KwEnum => self.parse_enum_def(token.span),
            TokenKind::KwExtend => self.parse_extend_def(token.span),
            _ => Err(ParseError::expected_item(token.kind, token.span)),
        }
    }

    pub(crate) fn expect(&mut self, kind: TokenKind, span: Span) -> PResult<Token> {
        let token = self
            .lexer
            .next()
            .ok_or_else(|| ParseError::unexpected_eof(Some(kind), span))?;
        if token.kind == TokenKind::Eof {
            return Err(ParseError::unexpected_eof(Some(kind), span));
        }
        if token.kind == kind {
            Ok(token)
        } else {
            Err(ParseError::expected(kind, token.kind, token.span))
        }
    }

    fn parse_import_def(&mut self, span: Span) -> PResult<ItemId> {
        let path = self.parse_path(span)?;
        let item_span = path
            .0
            .last()
            .map(|ident| span.merge(ident.span))
            .unwrap_or(span);
        let def = ImportDef { path };
        let item = Item {
            kind: ItemKind::Import(def),
            span: item_span,
        };
        Ok(self.ctx.alloc_item(item))
    }

    fn parse_path(&mut self, span: Span) -> PResult<ModPath> {
        let mut paths = Vec::new();
        let mut expected_ident_span = span;

        loop {
            let first = self.expect(TokenKind::Ident, expected_ident_span)?;
            let name = self.ctx.intern_str(&self.source[first.span.range()]);
            paths.push(Ident {
                name,
                span: first.span,
            });

            if let Some(separator) = self.lexer.next_if(|t| t.kind == TokenKind::ColonColon) {
                // If we hit EOF after `::`, report against the separator location.
                expected_ident_span = separator.span;
            } else {
                break;
            }
        }
        Ok(ModPath(paths))
    }

    fn parse_const_def(&mut self, const_span: Span) -> PResult<ItemId> {
        let name_tok = self.expect(TokenKind::Ident, const_span)?;
        let name = self.ctx.intern_str(&self.source[name_tok.span.range()]);
        let name_ident = Ident {
            name,
            span: name_tok.span,
        };

        let colon_tok = self.expect(TokenKind::Colon, name_tok.span)?;
        let ty_id = self.parse_type(colon_tok.span)?;
        let ty_span = self.ctx.get_type(ty_id).span;
        let eq_tok = self.expect(TokenKind::Assign, ty_span)?;
        if self
            .lexer
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Eof)
        {
            return Err(ParseError::unexpected_eof(None, eq_tok.span));
        }
        let expr_id = self.parse_expr()?;
        let expr_span = self.ctx.get_expr(expr_id).span;
        let item_span = const_span.merge(expr_span);

        let def = ConstDef {
            name: name_ident,
            ty: ty_id,
            value: expr_id,
        };
        let item = Item {
            kind: ItemKind::Const(def),
            span: item_span,
        };
        Ok(self.ctx.alloc_item(item))
    }

    fn parse_function_def(&mut self, fn_span: Span) -> PResult<ItemId> {
        let def = self.parse_function_def_inner(fn_span, false)?;
        let item_span = fn_span.merge(def.body.span);
        Ok(self.ctx.alloc_item(Item {
            kind: ItemKind::Function(def),
            span: item_span,
        }))
    }

    fn parse_function_def_inner(
        &mut self,
        fn_span: Span,
        allow_self_param: bool,
    ) -> PResult<FunctionDef> {
        let name_tok = self.expect(TokenKind::Ident, fn_span)?;
        let name = self.ident_from_token(&name_tok);
        let generic_params = self.parse_generic_params()?;
        let lparen = self.expect(TokenKind::LParen, name.span)?;
        let (params, _) = self.parse_params(lparen.span, allow_self_param)?;
        let return_type = if self
            .lexer
            .next_if(|token| token.kind == TokenKind::Arrow)
            .is_some()
        {
            let prev = self.prev_span();
            Some(self.parse_type(prev)?)
        } else {
            None
        };
        let body =
            self.parse_block(return_type.map_or(name.span, |ty| self.ctx.get_type(ty).span))?;
        Ok(FunctionDef {
            name,
            generic_params,
            params,
            return_type,
            body,
        })
    }

    fn parse_struct_def(&mut self, struct_span: Span) -> PResult<ItemId> {
        let name_tok = self.expect(TokenKind::Ident, struct_span)?;
        let name = self.ident_from_token(&name_tok);
        let generic_params = self.parse_generic_params()?;
        let lparen = self.expect(TokenKind::LParen, name.span)?;
        let (fields, rparen_span) = self.parse_params(lparen.span, false)?;
        let item_span = struct_span.merge(rparen_span);
        let def = StructDef {
            name,
            generic_params,
            fields,
        };
        Ok(self.ctx.alloc_item(Item {
            kind: ItemKind::Struct(def),
            span: item_span,
        }))
    }

    fn parse_enum_def(&mut self, enum_span: Span) -> PResult<ItemId> {
        let name_tok = self.expect(TokenKind::Ident, enum_span)?;
        let name = self.ident_from_token(&name_tok);
        let generic_params = self.parse_generic_params()?;
        let lbrace = self.expect(TokenKind::LBrace, name.span)?;
        let mut variants = Vec::new();

        if let Some(rbrace) = self.lexer.next_if(|token| token.kind == TokenKind::RBrace) {
            let def = EnumDef {
                name,
                generic_params,
                variants,
            };
            return Ok(self.ctx.alloc_item(Item {
                kind: ItemKind::Enum(def),
                span: enum_span.merge(rbrace.span),
            }));
        }

        loop {
            variants.push(self.parse_enum_variant()?);
            if self
                .lexer
                .next_if(|token| token.kind == TokenKind::Comma)
                .is_none()
            {
                break;
            }
            if self
                .lexer
                .peek()
                .is_some_and(|token| matches!(token.kind, TokenKind::RBrace | TokenKind::Eof))
            {
                break;
            }
        }

        let prev = variants
            .last()
            .and_then(|variant| {
                variant
                    .payload
                    .last()
                    .map(|ty| self.ctx.get_type(*ty).span)
                    .or(Some(variant.name.span))
            })
            .unwrap_or(lbrace.span);
        let rbrace = self.expect(TokenKind::RBrace, prev)?;
        let def = EnumDef {
            name,
            generic_params,
            variants,
        };
        Ok(self.ctx.alloc_item(Item {
            kind: ItemKind::Enum(def),
            span: enum_span.merge(rbrace.span),
        }))
    }

    fn parse_enum_variant(&mut self) -> PResult<EnumVariant> {
        let prev = self.prev_span();
        let name_tok = self.expect(TokenKind::Ident, prev)?;
        let name = self.ident_from_token(&name_tok);
        let mut payload = Vec::new();

        if self
            .lexer
            .next_if(|token| token.kind == TokenKind::LParen)
            .is_some()
        {
            loop {
                payload.push(self.parse_type(name.span)?);
                if self
                    .lexer
                    .next_if(|token| token.kind == TokenKind::Comma)
                    .is_none()
                {
                    break;
                }
            }
            let prev = payload
                .last()
                .map_or(name.span, |ty| self.ctx.get_type(*ty).span);
            self.expect(TokenKind::RParen, prev)?;
        }

        Ok(EnumVariant { name, payload })
    }

    fn parse_extend_def(&mut self, extend_span: Span) -> PResult<ItemId> {
        let target_tok = self.expect(TokenKind::Ident, extend_span)?;
        let target = self.ident_from_token(&target_tok);
        let generic_params = self.parse_generic_params()?;
        let lbrace = self.expect(TokenKind::LBrace, target.span)?;
        let mut methods = Vec::new();

        while !self
            .lexer
            .peek()
            .is_some_and(|token| matches!(token.kind, TokenKind::RBrace | TokenKind::Eof))
        {
            let fn_tok = self.expect(TokenKind::KwFn, lbrace.span)?;
            methods.push(self.parse_function_def_inner(fn_tok.span, true)?);
        }

        let prev = methods
            .last()
            .map_or(lbrace.span, |method| method.body.span);
        let rbrace = self.expect(TokenKind::RBrace, prev)?;
        Ok(self.ctx.alloc_item(Item {
            kind: ItemKind::Extend(ExtendDef {
                target,
                generic_params,
                methods,
            }),
            span: extend_span.merge(rbrace.span),
        }))
    }

    pub(crate) fn parse_type(&mut self, prev_span: Span) -> PResult<TypeId> {
        let path = self.parse_path(prev_span)?;
        let path_span = match (path.0.first(), path.0.last()) {
            (Some(first), Some(last)) => first.span.merge(last.span),
            _ => prev_span,
        };
        let (generic_args, span) = if self
            .lexer
            .next_if(|token| token.kind == TokenKind::Lt)
            .is_some()
        {
            let (generic_args, gt_span) = self.parse_type_args(path_span)?;
            (generic_args, path_span.merge(gt_span))
        } else {
            (Vec::new(), path_span)
        };
        let ty = Type {
            kind: TypeKind::Path { path, generic_args },
            span,
        };
        Ok(self.ctx.alloc_type(ty))
    }

    fn parse_generic_params(&mut self) -> PResult<Vec<Ident>> {
        if self
            .lexer
            .next_if(|token| token.kind == TokenKind::Lt)
            .is_none()
        {
            return Ok(Vec::new());
        }

        let mut params = Vec::new();
        loop {
            let prev = params
                .last()
                .map_or_else(|| self.prev_span(), |ident: &Ident| ident.span);
            let token = self.expect(TokenKind::Ident, prev)?;
            params.push(self.ident_from_token(&token));
            if self
                .lexer
                .next_if(|token| token.kind == TokenKind::Comma)
                .is_none()
            {
                break;
            }
        }
        let prev = params.last().map_or_else(|| self.prev_span(), |p| p.span);
        self.expect(TokenKind::Gt, prev)?;
        Ok(params)
    }

    fn parse_type_args(&mut self, prev_span: Span) -> PResult<(Vec<TypeId>, Span)> {
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type(prev_span)?);
            if self
                .lexer
                .next_if(|token| token.kind == TokenKind::Comma)
                .is_none()
            {
                break;
            }
        }
        let last_span = args
            .last()
            .map_or(prev_span, |ty| self.ctx.get_type(*ty).span);
        let gt = self.expect(TokenKind::Gt, last_span)?;
        Ok((args, gt.span))
    }

    fn parse_params(
        &mut self,
        lparen_span: Span,
        allow_self_param: bool,
    ) -> PResult<(Vec<Param>, Span)> {
        let mut params = Vec::new();
        if let Some(rparen) = self.lexer.next_if(|token| token.kind == TokenKind::RParen) {
            return Ok((params, rparen.span));
        }

        loop {
            params.push(self.parse_param(allow_self_param && params.is_empty())?);
            if self
                .lexer
                .next_if(|token| token.kind == TokenKind::Comma)
                .is_none()
            {
                break;
            }
            if let Some(rparen) = self.lexer.next_if(|token| token.kind == TokenKind::RParen) {
                return Ok((params, rparen.span));
            }
        }

        let prev = params
            .last()
            .map_or(lparen_span, |param| self.ctx.get_type(param.ty).span);
        let rparen = self.expect(TokenKind::RParen, prev)?;
        Ok((params, rparen.span))
    }

    fn parse_param(&mut self, allow_self_param: bool) -> PResult<Param> {
        let prev = self.prev_span();
        if allow_self_param
            && let Some(token) = self.lexer.next_if(|token| token.kind == TokenKind::KwSelf)
        {
            let name = self.ident_from_token(&token);
            let ty = self.ctx.alloc_type(Type {
                kind: TypeKind::Path {
                    path: ModPath(vec![name]),
                    generic_args: Vec::new(),
                },
                span: token.span,
            });
            return Ok(Param {
                label: ParamLabel::Implicit,
                name,
                ty,
            });
        }
        let first = self.expect(TokenKind::Ident, prev)?;
        let first_ident = self.ident_from_token(&first);

        let (label, name) = if self
            .lexer
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Ident)
        {
            let name_tok = self.expect(TokenKind::Ident, first.span)?;
            let label = if &self.source[first.span.range()] == "_" {
                ParamLabel::Suppressed
            } else {
                ParamLabel::Explicit(first_ident)
            };
            (label, self.ident_from_token(&name_tok))
        } else {
            (ParamLabel::Implicit, first_ident)
        };

        let colon = self.expect(TokenKind::Colon, name.span)?;
        let ty = self.parse_type(colon.span)?;
        Ok(Param { label, name, ty })
    }

    pub(crate) fn parse_block(&mut self, prev_span: Span) -> PResult<Block> {
        let lbrace = self.expect(TokenKind::LBrace, prev_span)?;
        let mut stmts = Vec::new();
        while !self
            .lexer
            .peek()
            .is_some_and(|token| matches!(token.kind, TokenKind::RBrace | TokenKind::Eof))
        {
            stmts.push(self.parse_stmt()?);
        }
        let rbrace = self.expect(TokenKind::RBrace, lbrace.span)?;
        Ok(Block {
            stmts,
            span: lbrace.span.merge(rbrace.span),
        })
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.lexer.peek().map(|token| token.kind) {
            Some(TokenKind::KwLet | TokenKind::KwVar) => self.parse_assignment_stmt(),
            Some(TokenKind::KwIf) => self.parse_if_stmt(),
            Some(TokenKind::KwFor) => self.parse_for_in_stmt(),
            Some(TokenKind::Ident | TokenKind::KwSelf) if self.next_starts_assignment() => {
                self.parse_assignment_stmt()
            }
            _ => {
                let expr = self.parse_expr()?;
                let span = self.ctx.get_expr(expr).span;
                Ok(Stmt {
                    kind: StmtKind::Expr(expr),
                    span,
                })
            }
        }
    }

    fn parse_assignment_stmt(&mut self) -> PResult<Stmt> {
        let first_span = self.prev_span();
        let binding = match self.lexer.peek().map(|token| token.kind) {
            Some(TokenKind::KwLet) => {
                self.lexer.next();
                Some(BindingKind::Let)
            }
            Some(TokenKind::KwVar) => {
                self.lexer.next();
                Some(BindingKind::Var)
            }
            _ => None,
        };
        let target = self.parse_assign_target()?;
        let target_span = match &target {
            AssignTarget::Ident(ident) => ident.span,
            AssignTarget::Field { base, fields } => fields
                .last()
                .map(|field| base.span.merge(field.span))
                .unwrap_or(base.span),
        };
        let ty = if self
            .lexer
            .next_if(|token| token.kind == TokenKind::Colon)
            .is_some()
        {
            Some(self.parse_type(target_span)?)
        } else {
            None
        };
        let prev = ty.map_or(target_span, |ty| self.ctx.get_type(ty).span);
        self.expect(TokenKind::Assign, prev)?;
        let value = self.parse_expr()?;
        let value_span = self.ctx.get_expr(value).span;
        Ok(Stmt {
            kind: StmtKind::Assignment(AssignmentStmt {
                binding,
                target,
                ty,
                value,
            }),
            span: first_span.merge(value_span),
        })
    }

    fn parse_assign_target(&mut self) -> PResult<AssignTarget> {
        let prev = self.prev_span();
        let first = self
            .lexer
            .next()
            .ok_or_else(|| ParseError::unexpected_eof(Some(TokenKind::Ident), prev))?;
        if !matches!(first.kind, TokenKind::Ident | TokenKind::KwSelf) {
            return Err(ParseError::expected(
                TokenKind::Ident,
                first.kind,
                first.span,
            ));
        }
        let base = self.ident_from_token(&first);
        let mut fields = Vec::new();
        while self
            .lexer
            .next_if(|token| token.kind == TokenKind::Dot)
            .is_some()
        {
            let field = self.expect(TokenKind::Ident, base.span)?;
            fields.push(self.ident_from_token(&field));
        }
        if fields.is_empty() {
            Ok(AssignTarget::Ident(base))
        } else {
            Ok(AssignTarget::Field { base, fields })
        }
    }

    fn parse_if_stmt(&mut self) -> PResult<Stmt> {
        let prev = self.prev_span();
        let if_tok = self.expect(TokenKind::KwIf, prev)?;
        let if_stmt = self.parse_if_stmt_after_if()?;
        let span = if_tok.span.merge(Self::if_stmt_end_span(&if_stmt));
        Ok(Stmt {
            kind: StmtKind::If(if_stmt),
            span,
        })
    }

    fn parse_if_stmt_after_if(&mut self) -> PResult<IfStmt> {
        let condition = self.parse_expr()?;
        let condition_span = self.ctx.get_expr(condition).span;
        let then_block = self.parse_block(condition_span)?;
        let else_branch = if self
            .lexer
            .next_if(|token| token.kind == TokenKind::KwElse)
            .is_some()
        {
            if self
                .lexer
                .next_if(|token| token.kind == TokenKind::KwIf)
                .is_some()
            {
                Some(ElseBranch::If(Box::new(self.parse_if_stmt_after_if()?)))
            } else {
                Some(ElseBranch::Block(self.parse_block(then_block.span)?))
            }
        } else {
            None
        };
        Ok(IfStmt {
            condition,
            then_block,
            else_branch,
        })
    }

    fn parse_for_in_stmt(&mut self) -> PResult<Stmt> {
        let prev = self.prev_span();
        let for_tok = self.expect(TokenKind::KwFor, prev)?;
        let binding_tok = self.expect(TokenKind::Ident, for_tok.span)?;
        let binding = self.ident_from_token(&binding_tok);
        self.expect(TokenKind::KwIn, binding.span)?;
        let iter = self.parse_expr()?;
        let iter_span = self.ctx.get_expr(iter).span;
        let body = self.parse_block(iter_span)?;
        let body_span = body.span;
        Ok(Stmt {
            kind: StmtKind::ForIn(ForInStmt {
                binding,
                iter,
                body,
            }),
            span: for_tok.span.merge(body_span),
        })
    }

    fn next_starts_assignment(&self) -> bool {
        let mut lookahead = self.lexer.clone();
        if !lookahead
            .next()
            .is_some_and(|token| matches!(token.kind, TokenKind::Ident | TokenKind::KwSelf))
        {
            return false;
        }
        loop {
            match lookahead.peek().map(|token| token.kind) {
                Some(TokenKind::Assign | TokenKind::Colon) => return true,
                Some(TokenKind::Dot) => {
                    lookahead.next();
                    if !lookahead
                        .next()
                        .is_some_and(|token| token.kind == TokenKind::Ident)
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        }
    }

    fn if_stmt_end_span(if_stmt: &IfStmt) -> Span {
        match &if_stmt.else_branch {
            Some(ElseBranch::If(else_if)) => Self::if_stmt_end_span(else_if),
            Some(ElseBranch::Block(block)) => block.span,
            None => if_stmt.then_block.span,
        }
    }

    pub(crate) fn ident_from_token(&mut self, token: &Token) -> Ident {
        let name = self.ctx.intern_str(&self.source[token.span.range()]);
        Ident {
            name,
            span: token.span,
        }
    }

    pub(crate) fn eof_span(&mut self) -> Span {
        self.lexer
            .peek()
            .map(|token| token.span)
            .unwrap_or_else(|| Span::new(self.source.len(), self.source.len(), FileId(0)))
    }

    fn prev_span(&mut self) -> Span {
        self.lexer
            .peek()
            .map(|token| token.span)
            .unwrap_or_else(|| Span::new(self.source.len(), self.source.len(), FileId(0)))
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use ast::{
        AstVisitor,
        ctx::AstCtx,
        expr::{ExprId, ExprKind, LitKind},
        item::{ConstDef, ImportDef, ItemId, StructDef},
        span::FileId,
        ty::TypeKind,
        walk_program,
    };
    use lexer::token::TokenKind;

    use super::Parser;
    use crate::error::{ParseError, ParseErrorKind};

    struct ParsedProgram {
        ctx: AstCtx,
        items: Vec<ItemId>,
    }

    impl ParsedProgram {
        fn compact_ast(&self) -> String {
            let mut formatter = CompactAstFormatter::new();
            walk_program(&mut formatter, &self.items, &self.ctx);
            formatter.into_output()
        }
    }

    struct CompactAstFormatter {
        out: String,
    }

    impl CompactAstFormatter {
        fn new() -> Self {
            Self { out: String::new() }
        }

        fn into_output(self) -> String {
            self.out
        }

        fn write_type(&mut self, ty: ast::ty::TypeId, ctx: &AstCtx) {
            match &ctx.get_type(ty).kind {
                TypeKind::Path { path, generic_args } => {
                    self.write_path(path, ctx);
                    if !generic_args.is_empty() {
                        self.out.push('<');
                        for (idx, generic_arg) in generic_args.iter().enumerate() {
                            if idx > 0 {
                                self.out.push_str(", ");
                            }
                            self.write_type(*generic_arg, ctx);
                        }
                        self.out.push('>');
                    }
                }
            }
        }

        fn write_path(&mut self, path: &ast::common::ModPath, ctx: &AstCtx) {
            for (idx, ident) in path.0.iter().enumerate() {
                if idx > 0 {
                    self.out.push_str("::");
                }
                self.out.push_str(ctx.get_str(ident.name));
            }
        }

        fn write_expr(&mut self, label: &str, expr_id: ExprId, ctx: &AstCtx) {
            let expr = ctx.get_expr(expr_id);
            match &expr.kind {
                ExprKind::Literal(lit) => match &lit.kind {
                    LitKind::Bool(value) => {
                        let _ = writeln!(&mut self.out, "{label}: bool {value}");
                    }
                    LitKind::Int => {
                        let _ = writeln!(&mut self.out, "{label}: int");
                    }
                    LitKind::Float => {
                        let _ = writeln!(&mut self.out, "{label}: float");
                    }
                    LitKind::Char => {
                        let _ = writeln!(&mut self.out, "{label}: char");
                    }
                    LitKind::Str => {
                        let _ = writeln!(&mut self.out, "{label}: str");
                    }
                },
                ExprKind::Path(path) => {
                    self.out.push_str(label);
                    self.out.push_str(": path ");
                    self.write_path(path, ctx);
                    self.out.push('\n');
                }
                ExprKind::ImplicitMember(member) => {
                    let _ = writeln!(
                        &mut self.out,
                        "{label}: implicit member {}",
                        ctx.get_str(member.name)
                    );
                }
                ExprKind::Unary(unary) => {
                    let _ = writeln!(&mut self.out, "{label}: unary {:?}", unary.op);
                    self.write_expr("  expr", unary.expr, ctx);
                }
                ExprKind::Binary(binary) => {
                    let _ = writeln!(&mut self.out, "{label}: binary {:?}", binary.op);
                    self.write_expr("  lhs", binary.lhs, ctx);
                    self.write_expr("  rhs", binary.rhs, ctx);
                }
                ExprKind::Call(call) => {
                    let _ = writeln!(&mut self.out, "{label}: call");
                    self.write_expr("  callee", call.callee, ctx);
                    for generic_arg in &call.generic_args {
                        self.out.push_str("  generic: ");
                        self.write_type(*generic_arg, ctx);
                        self.out.push('\n');
                    }
                    for arg in &call.args {
                        match &arg.label {
                            Some(label) => {
                                let _ = writeln!(
                                    &mut self.out,
                                    "  arg label: {}",
                                    ctx.get_str(label.name)
                                );
                            }
                            None => self.out.push_str("  arg\n"),
                        }
                        self.write_expr("    value", arg.value, ctx);
                    }
                }
                ExprKind::Field(field) => {
                    let _ = writeln!(
                        &mut self.out,
                        "{label}: field {}",
                        ctx.get_str(field.field.name)
                    );
                    self.write_expr("  base", field.base, ctx);
                }
                ExprKind::Array(array) => {
                    let _ = writeln!(&mut self.out, "{label}: array");
                    for element in &array.elements {
                        self.write_expr("  elem", *element, ctx);
                    }
                }
                ExprKind::If(if_expr) => {
                    let _ = writeln!(&mut self.out, "{label}: if");
                    self.write_expr("  cond", if_expr.condition, ctx);
                    for stmt in &if_expr.then_block.stmts {
                        if let ast::item::StmtKind::Expr(expr) = stmt.kind {
                            self.write_expr("  then", expr, ctx);
                        }
                    }
                }
            }
        }
    }

    impl AstVisitor for CompactAstFormatter {
        fn visit_import(&mut self, _item_id: ItemId, import: &ImportDef, ctx: &AstCtx) {
            self.out.push_str("import ");
            self.write_path(&import.path, ctx);
            self.out.push('\n');
        }

        fn visit_const(&mut self, _item_id: ItemId, def: &ConstDef, ctx: &AstCtx) {
            let _ = write!(&mut self.out, "const {}: ", ctx.get_str(def.name.name));
            self.write_type(def.ty, ctx);
            self.out.push('\n');
            self.write_expr("  value", def.value, ctx);
        }

        fn visit_function(&mut self, _item_id: ItemId, def: &ast::item::FunctionDef, ctx: &AstCtx) {
            let _ = write!(&mut self.out, "fn {}", ctx.get_str(def.name.name));
            if !def.generic_params.is_empty() {
                self.out.push('<');
                for (idx, generic) in def.generic_params.iter().enumerate() {
                    if idx > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(ctx.get_str(generic.name));
                }
                self.out.push('>');
            }
            self.out.push('(');
            for (idx, param) in def.params.iter().enumerate() {
                if idx > 0 {
                    self.out.push_str(", ");
                }
                match &param.label {
                    ast::item::ParamLabel::Implicit => {}
                    ast::item::ParamLabel::Explicit(label) => {
                        self.out.push_str(ctx.get_str(label.name));
                        self.out.push(' ');
                    }
                    ast::item::ParamLabel::Suppressed => self.out.push_str("_ "),
                }
                self.out.push_str(ctx.get_str(param.name.name));
                self.out.push_str(": ");
                self.write_type(param.ty, ctx);
            }
            self.out.push(')');
            if let Some(return_type) = def.return_type {
                self.out.push_str(" -> ");
                self.write_type(return_type, ctx);
            }
            self.out.push('\n');
            for stmt in &def.body.stmts {
                match &stmt.kind {
                    ast::item::StmtKind::Expr(expr) => self.write_expr("  expr", *expr, ctx),
                    ast::item::StmtKind::Assignment(assign) => {
                        let _ = writeln!(&mut self.out, "  assign");
                        self.write_expr("    value", assign.value, ctx);
                    }
                    ast::item::StmtKind::If(if_stmt) => {
                        let _ = writeln!(&mut self.out, "  if");
                        self.write_expr("    cond", if_stmt.condition, ctx);
                    }
                    ast::item::StmtKind::ForIn(for_in) => {
                        let _ =
                            writeln!(&mut self.out, "  for {}", ctx.get_str(for_in.binding.name));
                    }
                }
            }
        }

        fn visit_struct(&mut self, _item_id: ItemId, def: &StructDef, ctx: &AstCtx) {
            let _ = write!(&mut self.out, "struct {}", ctx.get_str(def.name.name));
            if !def.generic_params.is_empty() {
                self.out.push('<');
                for (idx, generic) in def.generic_params.iter().enumerate() {
                    if idx > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(ctx.get_str(generic.name));
                }
                self.out.push('>');
            }
            self.out.push('(');
            for (idx, field) in def.fields.iter().enumerate() {
                if idx > 0 {
                    self.out.push_str(", ");
                }
                match &field.label {
                    ast::item::ParamLabel::Implicit => {}
                    ast::item::ParamLabel::Explicit(label) => {
                        self.out.push_str(ctx.get_str(label.name));
                        self.out.push(' ');
                    }
                    ast::item::ParamLabel::Suppressed => self.out.push_str("_ "),
                }
                self.out.push_str(ctx.get_str(field.name.name));
                self.out.push_str(": ");
                self.write_type(field.ty, ctx);
            }
            self.out.push_str(")\n");
        }

        fn visit_enum(&mut self, _item_id: ItemId, def: &ast::item::EnumDef, ctx: &AstCtx) {
            let _ = write!(&mut self.out, "enum {}", ctx.get_str(def.name.name));
            if !def.generic_params.is_empty() {
                self.out.push('<');
                for (idx, generic) in def.generic_params.iter().enumerate() {
                    if idx > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(ctx.get_str(generic.name));
                }
                self.out.push('>');
            }
            self.out.push_str(" {");
            for (idx, variant) in def.variants.iter().enumerate() {
                if idx > 0 {
                    self.out.push_str(", ");
                }
                self.out.push_str(ctx.get_str(variant.name.name));
                if !variant.payload.is_empty() {
                    self.out.push('(');
                    for (idx, ty) in variant.payload.iter().enumerate() {
                        if idx > 0 {
                            self.out.push_str(", ");
                        }
                        self.write_type(*ty, ctx);
                    }
                    self.out.push(')');
                }
            }
            self.out.push_str("}\n");
        }

        fn visit_extend(&mut self, _item_id: ItemId, def: &ast::item::ExtendDef, ctx: &AstCtx) {
            let _ = write!(&mut self.out, "extend {}", ctx.get_str(def.target.name));
            if !def.generic_params.is_empty() {
                self.out.push('<');
                for (idx, generic) in def.generic_params.iter().enumerate() {
                    if idx > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(ctx.get_str(generic.name));
                }
                self.out.push('>');
            }
            self.out.push('\n');
            for method in &def.methods {
                let _ = writeln!(&mut self.out, "  method {}", ctx.get_str(method.name.name));
            }
        }
    }

    fn parse(source: &str) -> (AstCtx, Result<Vec<ItemId>, ParseError>) {
        let mut ctx = AstCtx::new();
        let mut parser = Parser::new(&mut ctx, source, FileId(0));
        let result = parser.parse_program();
        (ctx, result)
    }

    fn parse_ok(source: &str) -> ParsedProgram {
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept source");
        ParsedProgram { ctx, items }
    }

    fn parse_err(source: &str) -> ParseError {
        let (_ctx, result) = parse(source);
        result.expect_err("parser should reject source")
    }

    #[test]
    fn parses_single_segment_import() {
        let parsed = parse_ok("import std");

        assert_eq!(parsed.compact_ast(), "import std\n");
    }

    #[test]
    fn parses_multi_segment_import_and_merges_item_span() {
        let source = "import std::Option";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept source");
        assert_eq!(items.len(), 1);

        let item = ctx.get_item(items[0]);
        assert_eq!(item.span.start, 0);
        assert_eq!(item.span.end, source.len());
        assert_eq!(
            ParsedProgram { ctx, items }.compact_ast(),
            "import std::Option\n"
        );
    }

    #[test]
    fn errors_on_missing_import_path() {
        let err = parse_err("import");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, 6);
    }

    #[test]
    fn errors_on_trailing_path_separator_at_separator_span() {
        let err = parse_err("import std::");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 10);
        assert_eq!(err.span.end, 12);
        assert_eq!(err.span.file, FileId(0));
    }

    #[test]
    fn errors_on_non_ident_after_separator() {
        let err = parse_err("import std::1");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Integer
            }
        ));
        assert_eq!(err.span.start, 12);
        assert_eq!(err.span.end, 13);
    }

    #[test]
    fn parses_const_float() {
        let parsed = parse_ok("const PI: f64 = 3.14");

        assert_eq!(parsed.compact_ast(), "const PI: f64\n  value: float\n");
    }

    #[test]
    fn parses_const_str() {
        let parsed = parse_ok(r#"const NAME: str = "abc""#);

        assert_eq!(parsed.compact_ast(), "const NAME: str\n  value: str\n");
    }

    #[test]
    fn parses_const_bool_true() {
        let parsed = parse_ok("const FLAG: bool = true");

        assert_eq!(
            parsed.compact_ast(),
            "const FLAG: bool\n  value: bool true\n"
        );
    }

    #[test]
    fn parses_mixed_consts_and_import() {
        let parsed = parse_ok("const A: i32 = 1\nimport std\nconst B: str = \"x\"");

        assert_eq!(
            parsed.compact_ast(),
            "const A: i32\n  value: int\nimport std\nconst B: str\n  value: str\n"
        );
    }

    #[test]
    fn parses_expr_precedence() {
        let parsed = parse_ok("const X: i32 = 1 + 2 * 3");

        assert_eq!(
            parsed.compact_ast(),
            "const X: i32\n  value: binary Add\n  lhs: int\n  rhs: binary Mul\n  lhs: int\n  rhs: int\n"
        );
    }

    #[test]
    fn parses_unary_field_and_call() {
        let parsed = parse_ok("const X: bool = !foo.bar::<T>(a: 1, 2)");

        assert_eq!(
            parsed.compact_ast(),
            "const X: bool\n  value: unary Not\n  expr: call\n  callee: field bar\n  base: path foo\n  generic: T\n  arg label: a\n    value: int\n  arg\n    value: int\n"
        );
    }

    #[test]
    fn errors_on_const_missing_name() {
        let err = parse_err("const");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, 5);
    }

    #[test]
    fn errors_on_const_missing_colon() {
        let err = parse_err("const X");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Colon)
            }
        ));
        assert_eq!(err.span.start, 6);
        assert_eq!(err.span.end, 7);
    }

    #[test]
    fn errors_on_const_missing_type_after_colon() {
        let err = parse_err("const X:");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 7);
        assert_eq!(err.span.end, 8);
    }

    #[test]
    fn errors_on_const_missing_assign() {
        let err = parse_err("const X: T");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Assign)
            }
        ));
        assert_eq!(err.span.start, 9);
        assert_eq!(err.span.end, 10);
    }

    #[test]
    fn errors_on_const_missing_rhs() {
        let err = parse_err("const X: T =");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof { expected: None }
        ));
        let source = "const X: T =";
        assert_eq!(err.span.start, source.len() - 1);
        assert_eq!(err.span.end, source.len());
    }

    #[test]
    fn errors_on_const_non_literal_rhs() {
        let err = parse_err("const X: T = +");
        assert!(matches!(
            err.kind,
            ParseErrorKind::ExpectedExpr {
                found: TokenKind::Plus
            }
        ));
        let source = "const X: T = +";
        let plus = source.rfind('+').expect("plus");
        assert_eq!(err.span.start, plus);
        assert_eq!(err.span.end, plus + 1);
    }

    #[test]
    fn errors_on_const_integer_as_name() {
        let err = parse_err("const 1: T = 0");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Integer
            }
        ));
        assert_eq!(err.span.start, 6);
        assert_eq!(err.span.end, 7);
    }

    #[test]
    fn parses_const_multi_segment_type_path() {
        let source = "const OPT: std::Option = 1";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept source");
        assert_eq!(items.len(), 1);

        let item = ctx.get_item(items[0]);
        let ast::item::ItemKind::Const(def) = &item.kind else {
            panic!("expected const");
        };
        let ty = ctx.get_type(def.ty);
        let TypeKind::Path { path, generic_args } = &ty.kind;
        assert!(generic_args.is_empty());
        assert_eq!(path.0.len(), 2);
        assert_eq!(ctx.get_str(path.0[0].name), "std");
        assert_eq!(ctx.get_str(path.0[1].name), "Option");
        let type_snippet = "std::Option";
        let start = source.find(type_snippet).expect("type snippet");
        assert_eq!(ty.span.start, start);
        assert_eq!(ty.span.end, start + type_snippet.len());
        assert_eq!(&source[ty.span.range()], type_snippet);
        assert_eq!(
            ParsedProgram { ctx, items }.compact_ast(),
            "const OPT: std::Option\n  value: int\n"
        );
    }

    #[test]
    fn parses_empty_struct() {
        let parsed = parse_ok("struct Empty()");

        assert_eq!(parsed.compact_ast(), "struct Empty()\n");
    }

    #[test]
    fn parses_struct_fields_with_trailing_comma() {
        let parsed = parse_ok("struct Point(x: f32, y: f32,)");

        assert_eq!(parsed.compact_ast(), "struct Point(x: f32, y: f32)\n");
    }

    #[test]
    fn parses_generic_struct_with_generic_field_type() {
        let parsed = parse_ok("struct Box<T>(value: Option<T>)");

        assert_eq!(parsed.compact_ast(), "struct Box<T>(value: Option<T>)\n");
    }

    #[test]
    fn parses_struct_field_labels() {
        let parsed = parse_ok("struct P(public x: f32, _ y: f32)");

        assert_eq!(parsed.compact_ast(), "struct P(public x: f32, _ y: f32)\n");
    }

    #[test]
    fn parses_mixed_items_with_struct() {
        let parsed = parse_ok("import std\nstruct Empty()\nconst X: i32 = 1\nfn main() {}");

        assert_eq!(
            parsed.compact_ast(),
            "import std\nstruct Empty()\nconst X: i32\n  value: int\nfn main()\n"
        );
    }

    #[test]
    fn parses_empty_enum() {
        let parsed = parse_ok("enum Void {}");

        assert_eq!(parsed.compact_ast(), "enum Void {}\n");
    }

    #[test]
    fn parses_enum_variants_payloads_generics_and_trailing_comma() {
        let parsed = parse_ok("enum Option<T> { some(T), none, }");

        assert_eq!(parsed.compact_ast(), "enum Option<T> {some(T), none}\n");
    }

    #[test]
    fn parses_explicit_enum_member_call() {
        let parsed = parse_ok("const X: Option = Option.some(1)");

        assert_eq!(
            parsed.compact_ast(),
            "const X: Option\n  value: call\n  callee: field some\n  base: path Option\n  arg\n    value: int\n"
        );
    }

    #[test]
    fn parses_qualified_enum_member_call() {
        let parsed = parse_ok("const X: Option = std::Option.some(1)");

        assert_eq!(
            parsed.compact_ast(),
            "const X: Option\n  value: call\n  callee: field some\n  base: path std::Option\n  arg\n    value: int\n"
        );
    }

    #[test]
    fn parses_implicit_enum_member_call_and_value() {
        let parsed = parse_ok("const A: Option = .some(1)\nconst B: Option = .none");

        assert_eq!(
            parsed.compact_ast(),
            "const A: Option\n  value: call\n  callee: implicit member some\n  arg\n    value: int\nconst B: Option\n  value: implicit member none\n"
        );
    }

    #[test]
    fn errors_on_enum_missing_variant_name() {
        let err = parse_err("enum E { 1 }");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Integer
            }
        ));
    }

    #[test]
    fn errors_on_enum_missing_payload_type() {
        let err = parse_err("enum E { some(, none }");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Comma
            }
        ));
    }

    #[test]
    fn errors_on_empty_enum_payload() {
        let err = parse_err("enum E { none() }");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::RParen
            }
        ));
    }

    #[test]
    fn errors_on_dangling_dot_expr() {
        let err = parse_err("const X: T = .");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
    }

    #[test]
    fn struct_span_ends_at_close_paren() {
        let source = "struct Empty()\nfn main() {}";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept source");
        let item = ctx.get_item(items[0]);

        assert_eq!(item.span.start, 0);
        assert_eq!(item.span.end, "struct Empty()".len());
    }

    #[test]
    fn errors_on_struct_missing_name() {
        let err = parse_err("struct");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
    }

    #[test]
    fn errors_on_struct_missing_paren() {
        let err = parse_err("struct Empty");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::LParen)
            }
        ));
    }

    #[test]
    fn errors_on_bad_struct_field() {
        let err = parse_err("struct P(1: T)");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Integer
            }
        ));
    }

    #[test]
    fn errors_on_struct_missing_close_paren() {
        let err = parse_err("struct P(x: T");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::RParen)
            }
        ));
    }

    #[test]
    fn errors_on_const_trailing_colon_colon_in_type() {
        let err = parse_err("const X: std:: = 1");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Assign
            }
        ));
        let source = "const X: std:: = 1";
        let eq = source.find('=').expect("=");
        assert_eq!(err.span.start, eq);
        assert_eq!(err.span.end, eq + 1);
    }

    #[test]
    fn parses_empty_function() {
        let parsed = parse_ok("fn main() {}");

        assert_eq!(parsed.compact_ast(), "fn main()\n");
    }

    #[test]
    fn parses_function_params() {
        let parsed = parse_ok("fn calc(a: usize, x b: i32, _ c: bool,) {}");

        assert_eq!(
            parsed.compact_ast(),
            "fn calc(a: usize, x b: i32, _ c: bool)\n"
        );
    }

    #[test]
    fn parses_generic_function_with_return_and_body_expr() {
        let parsed = parse_ok("fn id<T>(x: T) -> T { x }");

        assert_eq!(
            parsed.compact_ast(),
            "fn id<T>(x: T) -> T\n  expr: path x\n"
        );
    }

    #[test]
    fn parses_generic_type_args_in_function_param() {
        let parsed = parse_ok("fn f<T>(x: Option<T>) {}");

        assert_eq!(parsed.compact_ast(), "fn f<T>(x: Option<T>)\n");
    }

    #[test]
    fn parses_multi_expr_function_body() {
        let parsed = parse_ok("fn f() { 1 2 + 3 }");

        assert_eq!(
            parsed.compact_ast(),
            "fn f()\n  expr: int\n  expr: binary Add\n  lhs: int\n  rhs: int\n"
        );
    }

    #[test]
    fn errors_on_function_missing_name() {
        let err = parse_err("fn");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
    }

    #[test]
    fn errors_on_function_missing_paren() {
        let err = parse_err("fn main {}");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::LParen,
                found: TokenKind::LBrace
            }
        ));
    }

    #[test]
    fn errors_on_bad_function_param() {
        let err = parse_err("fn f(1: T) {}");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Integer
            }
        ));
    }

    #[test]
    fn errors_on_function_missing_close_brace() {
        let err = parse_err("fn f() { 1");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::RBrace)
            }
        ));
    }

    #[test]
    fn errors_on_bad_function_return_type() {
        let err = parse_err("fn f() -> { }");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::LBrace
            }
        ));
    }

    #[test]
    fn parses_extend_with_self_method() {
        let parsed = parse_ok("extend Point { fn f(self) {} }");

        assert_eq!(parsed.compact_ast(), "extend Point\n  method f\n");
    }

    #[test]
    fn parses_assignment_statements() {
        let parsed = parse_ok("fn f() { let a = 1 var b: T = x a = 2 self.x = 3 }");

        assert_eq!(
            parsed.compact_ast(),
            "fn f()\n  assign\n    value: int\n  assign\n    value: path x\n  assign\n    value: int\n  assign\n    value: int\n"
        );
    }

    #[test]
    fn parses_if_statement_chain() {
        let parsed = parse_ok("fn f() { if i > 0 { 1 } else if i < 0 { 2 } else { 3 } }");

        assert_eq!(
            parsed.compact_ast(),
            "fn f()\n  if\n    cond: binary Gt\n  lhs: path i\n  rhs: int\n"
        );
    }

    #[test]
    fn parses_if_expression() {
        let parsed = parse_ok("fn f() { let x = if cond { 1 } else { 2 } }");

        assert_eq!(
            parsed.compact_ast(),
            "fn f()\n  assign\n    value: if\n  cond: path cond\n  then: int\n"
        );
    }

    #[test]
    fn parses_for_in_statement() {
        let parsed = parse_ok("fn f() { for x in xs { x } }");

        assert_eq!(parsed.compact_ast(), "fn f()\n  for x\n");
    }

    #[test]
    fn parses_array_literals() {
        let parsed = parse_ok("fn f() { [] [1, 2, 3,] }");

        assert_eq!(
            parsed.compact_ast(),
            "fn f()\n  expr: array\n  expr: array\n  elem: int\n  elem: int\n  elem: int\n"
        );
    }

    #[test]
    fn parses_kitchen_sink_asset() {
        parse_ok(include_str!("../../../assets/kitchen_sink.pr"));
    }
}
