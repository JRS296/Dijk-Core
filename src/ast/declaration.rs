/// The root of a parsed Dijk program: every top-level declaration in the
/// order it appeared in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

/// A top-level declaration. `Function` is the only kind Dijk Core
/// supports today; structs and enums are expected to join this enum once
/// the grammar grows to include them.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Function(Function),
}

/// A function declaration.
///
/// This is intentionally minimal for now — parameters, a return type, and
/// a body are added in the next step of this milestone (see
/// `docs/roadmap.md`'s commit progression for `dijk-0.0.3`).
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
}
