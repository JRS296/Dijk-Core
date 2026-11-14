//! Renders Dijk's AST as an indented tree, for `dijk parse` and
//! `dijk ast`, and what the type checker learned about a program, for
//! `dijk check --types`. This is presentation only — it has no bearing on
//! the AST itself (`src/ast/`) or on the checker (`src/types/`).

use crate::ast::declaration::{Declaration, Function, Program};
use crate::ast::expression::Expression;
use crate::ast::statement::Statement;
use crate::ast::visit::{self, Visitor};
use crate::diagnostics;
use crate::types::{TypedFunction, TypedProgram};

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
            mutable,
            name,
            type_annotation,
            value,
            ..
        } => {
            let mut children = vec![Doc::Leaf(format!("Name: {name}"))];
            // Shown only for `let mut`, so that an immutable binding, the
            // default, prints exactly as it always has.
            if *mutable {
                children.push(Doc::Leaf("Mutable: true".to_string()));
            }
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
        Expression::Call {
            callee, arguments, ..
        } => Doc::Node(
            format!("Call({callee})"),
            arguments.iter().map(expr_doc).collect(),
        ),
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

/// The typing environment of every function, as `dijk check --types`
/// prints it: the function's signature, then every variable it declares
/// (parameters first, then each `let` in source order) with the type the
/// checker gave it and where its name was written:
///
/// ```text
/// fn add(a: i32, b: i32) -> i32
///     a      : i32   (1:8)
///     b      : i32   (1:16)
///     result : i32   (2:9)
/// fn main() -> i32
/// ```
///
/// Each line is one entry of Γ, `name : type`, as it stood where the
/// variable was declared. A variable declared inside a block is indented
/// one step further per level of nesting, so a shadowing variable appears
/// as a separate, more deeply indented entry. A `let mut` binding is
/// marked `mut` at the end of its line: mutability is a property of the
/// binding, not part of its type. A function without `-> T` is shown with
/// the return type it really has, `-> unit`.
pub fn print_types(typed: &TypedProgram, source: &str) -> String {
    let mut out = String::new();
    for function in &typed.functions {
        out.push_str(&signature_line(function));
        out.push('\n');
        out.push_str(&binding_lines(function, source));
    }
    out
}

fn signature_line(function: &TypedFunction) -> String {
    let params = function
        .signature
        .params
        .iter()
        .map(|(name, ty)| format!("{name}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "fn {}({params}) -> {}",
        function.name, function.signature.return_type
    )
}

/// One line per binding, in aligned columns: name, type, position.
fn binding_lines(function: &TypedFunction, source: &str) -> String {
    let rows: Vec<(String, String, String, bool)> = function
        .bindings
        .iter()
        .map(|binding| {
            let indent = "    ".repeat(binding.depth + 1);
            let (line, column) = diagnostics::line_and_column(source, binding.span.start);
            (
                format!("{indent}{}", binding.name),
                binding.ty.to_string(),
                format!("({line}:{column})"),
                binding.mutable,
            )
        })
        .collect();

    let name_width = rows.iter().map(|row| row.0.len()).max().unwrap_or(0);
    let type_width = rows.iter().map(|row| row.1.len()).max().unwrap_or(0);

    let mut out = String::new();
    for (name, ty, position, mutable) in rows {
        out.push_str(&format!(
            "{name:<name_width$} : {ty:<type_width$}   {position}"
        ));
        if mutable {
            out.push_str("  mut");
        }
        out.push('\n');
    }
    out
}
