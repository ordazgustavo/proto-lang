use ast::{common::Ident, span::Span};

#[derive(Debug, Clone)]
pub struct HirDiagnostic {
    pub kind: HirDiagnosticKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum HirDiagnosticKind {
    DuplicateTopLevelValue { name: Ident, first: Span },
    DuplicateTopLevelType { name: Ident, first: Span },
    DuplicateParam { name: Ident, first: Span },
    DuplicateLocal { name: Ident, first: Span },
    DuplicateField { name: Ident, first: Span },
    DuplicateVariant { name: Ident, first: Span },
    DuplicateGenericParam { name: Ident, first: Span },
    DuplicateMethod { name: Ident, first: Span },
}

impl HirDiagnostic {
    pub fn new(kind: HirDiagnosticKind, span: Span) -> Self {
        Self { kind, span }
    }
}
