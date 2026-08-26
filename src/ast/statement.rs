use super::expression::Expression;
use super::types::Type;

/// A statement: something executed for effect, rather than evaluated for
/// a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let { name: String, type_annotation: Option<Type>, value: Expression },
    Return(Option<Expression>),
    Expression(Expression),
    /// `else if` is not a distinct case: `else_branch` for an `else if`
    /// is simply a block containing exactly one `If` statement. There is
    /// nothing about "else if" that the AST needs to represent specially
    /// — it's surface syntax over the general shape below.
    If { condition: Expression, then_branch: Vec<Statement>, else_branch: Option<Vec<Statement>> },
    While { condition: Expression, body: Vec<Statement> },
}
