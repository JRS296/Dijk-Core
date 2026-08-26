//! Renders Dijk's AST as an indented tree, for `dijk parse` and
//! `dijk ast`. This is presentation only — it has no bearing on the AST
//! itself (`src/ast/`).

use crate::ast::declaration::{Declaration, Function, Program};
use crate::ast::expression::Expression;
use crate::ast::statement::Statement;
use crate::ast::visit::{self, Visitor};

enum Doc {
    Leaf(String),
    Node(String, Vec<Doc>),
}

pub fn print_program(program: &Program) -> String {
    let doc = Doc::Node(
        "Program".to_string(),
        program.declarations.iter().map(declaration_doc).collect(),
    );
    render(&doc)
}

/// How many functions, statements, and expressions a program contains,
/// as counted by walking it with [`Visitor`] — a small, real use of the
/// traversal machinery in `crate::ast::visit`, for `dijk ast`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NodeCounts {
    pub functions: usize,
    pub statements: usize,
    pub expressions: usize,
}

impl Visitor for NodeCounts {
    fn visit_function(&mut self, function: &Function) {
        self.functions += 1;
        visit::walk_function(self, function);
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        self.statements += 1;
        visit::walk_statement(self, stmt);
    }

    fn visit_expression(&mut self, expr: &Expression) {
        self.expressions += 1;
        visit::walk_expression(self, expr);
    }
}

pub fn count_nodes(program: &Program) -> NodeCounts {
    let mut counts = NodeCounts::default();
    counts.visit_program(program);
    counts
}

fn declaration_doc(decl: &Declaration) -> Doc {
    match decl {
        Declaration::Function(f) => function_doc(f),
    }
}

fn function_doc(f: &Function) -> Doc {
    let params = f
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty.name))
        .collect::<Vec<_>>()
        .join(", ");

    let mut children = vec![
        Doc::Leaf(format!("Name: {}", f.name)),
        Doc::Leaf(format!("Parameters: [{params}]")),
    ];

    if let Some(ret) = &f.return_type {
        children.push(Doc::Leaf(format!("Return type: {}", ret.name)));
    }

    children.push(Doc::Node("Body".to_string(), block_docs(&f.body)));
    Doc::Node("Function".to_string(), children)
}

fn block_docs(statements: &[Statement]) -> Vec<Doc> {
    statements.iter().map(stmt_doc).collect()
}

fn stmt_doc(stmt: &Statement) -> Doc {
    match stmt {
        Statement::Let {
            name,
            type_annotation,
            value,
            ..
        } => {
            let mut children = vec![Doc::Leaf(format!("Name: {name}"))];
            if let Some(ty) = type_annotation {
                children.push(Doc::Leaf(format!("Type: {}", ty.name)));
            }
            children.push(Doc::Node("Value".to_string(), vec![expr_doc(value)]));
            Doc::Node("Let".to_string(), children)
        }
        Statement::Return { value, .. } => {
            let children = value
                .as_ref()
                .map(|v| vec![expr_doc(v)])
                .unwrap_or_default();
            Doc::Node("Return".to_string(), children)
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let mut children = vec![
                Doc::Node("Condition".to_string(), vec![expr_doc(condition)]),
                Doc::Node("Then".to_string(), block_docs(then_branch)),
            ];
            if let Some(statements) = else_branch {
                children.push(Doc::Node("Else".to_string(), block_docs(statements)));
            }
            Doc::Node("If".to_string(), children)
        }
        Statement::While {
            condition, body, ..
        } => Doc::Node(
            "While".to_string(),
            vec![
                Doc::Node("Condition".to_string(), vec![expr_doc(condition)]),
                Doc::Node("Body".to_string(), block_docs(body)),
            ],
        ),
        Statement::Expression { expr, .. } => {
            Doc::Node("ExpressionStatement".to_string(), vec![expr_doc(expr)])
        }
    }
}

fn expr_doc(expr: &Expression) -> Doc {
    match expr {
        Expression::Integer(value, _) => Doc::Leaf(format!("Integer({value})")),
        Expression::String(value, _) => Doc::Leaf(format!("String({value:?})")),
        Expression::Boolean(value, _) => Doc::Leaf(format!("Boolean({value})")),
        Expression::Identifier(name, _) => Doc::Leaf(format!("Identifier({name})")),
        Expression::Unary {
            operator, operand, ..
        } => Doc::Node(format!("Unary({operator})"), vec![expr_doc(operand)]),
        Expression::Binary {
            operator,
            left,
            right,
            ..
        } => Doc::Node(
            format!("Binary({operator})"),
            vec![expr_doc(left), expr_doc(right)],
        ),
        Expression::Assign { name, value, .. } => {
            Doc::Node(format!("Assign({name})"), vec![expr_doc(value)])
        }
    }
}

fn render(doc: &Doc) -> String {
    let mut out = String::new();
    match doc {
        Doc::Leaf(text) => {
            out.push_str(text);
            out.push('\n');
        }
        Doc::Node(text, children) => {
            out.push_str(text);
            out.push('\n');
            render_children(&mut out, children, "");
        }
    }
    out
}

fn render_children(out: &mut String, children: &[Doc], prefix: &str) {
    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });

        match child {
            Doc::Leaf(text) => {
                out.push_str(prefix);
                out.push_str(connector);
                out.push_str(text);
                out.push('\n');
            }
            Doc::Node(text, grandchildren) => {
                out.push_str(prefix);
                out.push_str(connector);
                out.push_str(text);
                out.push('\n');
                render_children(out, grandchildren, &child_prefix);
            }
        }
    }
}
