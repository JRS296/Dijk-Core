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
    Integer(i64),
    String(String),
    Boolean(bool),
    Identifier(String),
    Binary { left: Box<Expression>, operator: BinaryOperator, right: Box<Expression> },
    Unary { operator: UnaryOperator, operand: Box<Expression> },
    /// An assignment to a plain identifier. Dijk Core's grammar currently
    /// allows no other assignment target.
    Assign { name: String, value: Box<Expression> },
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
