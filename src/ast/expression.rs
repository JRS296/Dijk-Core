use crate::lexer::token::Span;
use std::fmt;

/// An expression: something evaluated for a value.
///
/// Unlike the parser's provisional syntax representation, there is no
/// `Grouping` node here. `(a + b) * c` and an expression that was simply
/// written with `+` already nested under `*` produce the exact same
/// tree — parentheses are surface syntax that exists only to guide
/// parsing, and once precedence has been resolved they carry no further
/// meaning. This is the concrete illustration of "parse tree vs. AST"
/// (see `docs/ast.md`).
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64, Span),
    String(String, Span),
    Boolean(bool, Span),
    Identifier(String, Span),
    Binary { left: Box<Expression>, operator: BinaryOperator, right: Box<Expression>, span: Span },
    Unary { operator: UnaryOperator, operand: Box<Expression>, span: Span },
    /// An assignment to a plain identifier. Dijk Core's grammar currently
    /// allows no other assignment target.
    Assign { name: String, value: Box<Expression>, span: Span },
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Integer(_, span)
            | Expression::String(_, span)
            | Expression::Boolean(_, span)
            | Expression::Identifier(_, span)
            | Expression::Binary { span, .. }
            | Expression::Unary { span, .. }
            | Expression::Assign { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
            BinaryOperator::Eq => "==",
            BinaryOperator::NotEq => "!=",
            BinaryOperator::Lt => "<",
            BinaryOperator::Le => "<=",
            BinaryOperator::Gt => ">",
            BinaryOperator::Ge => ">=",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Neg,
    Not,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnaryOperator::Neg => "-",
            UnaryOperator::Not => "!",
        })
    }
}
