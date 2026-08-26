//! Renders the parser's syntax representation as an indented tree, for
//! `dijk parse`. This is presentation only — it has no bearing on the
//! syntax representation itself (`src/parser/syntax.rs`).

use crate::parser::syntax::{Block, ElseBranch, Expr, Function, Program, Stmt};

enum Doc {
    Leaf(String),
    Node(String, Vec<Doc>),
}

pub fn print_program(program: &Program) -> String {
    let doc = Doc::Node(
        "Program".to_string(),
        program.functions.iter().map(function_doc).collect(),
    );
    render(&doc)
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

fn block_docs(block: &Block) -> Vec<Doc> {
    block.statements.iter().map(stmt_doc).collect()
}

fn stmt_doc(stmt: &Stmt) -> Doc {
    match stmt {
        Stmt::Let {
            name, ty, value, ..
        } => {
            let mut children = vec![Doc::Leaf(format!("Name: {name}"))];
            if let Some(ty) = ty {
                children.push(Doc::Leaf(format!("Type: {}", ty.name)));
            }
            children.push(Doc::Node("Value".to_string(), vec![expr_doc(value)]));
            Doc::Node("Let".to_string(), children)
        }
        Stmt::Return { value, .. } => {
            let children = value
                .as_ref()
                .map(|v| vec![expr_doc(v)])
                .unwrap_or_default();
            Doc::Node("Return".to_string(), children)
        }
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            let mut children = vec![
                Doc::Node("Condition".to_string(), vec![expr_doc(condition)]),
                Doc::Node("Then".to_string(), block_docs(then_block)),
            ];
            if let Some(branch) = else_branch {
                children.push(match branch {
                    ElseBranch::Block(block) => Doc::Node("Else".to_string(), block_docs(block)),
                    ElseBranch::If(nested) => Doc::Node("Else".to_string(), vec![stmt_doc(nested)]),
                });
            }
            Doc::Node("If".to_string(), children)
        }
        Stmt::While {
            condition, body, ..
        } => Doc::Node(
            "While".to_string(),
            vec![
                Doc::Node("Condition".to_string(), vec![expr_doc(condition)]),
                Doc::Node("Body".to_string(), block_docs(body)),
            ],
        ),
        Stmt::Expression { expr, .. } => {
            Doc::Node("ExpressionStatement".to_string(), vec![expr_doc(expr)])
        }
    }
}

fn expr_doc(expr: &Expr) -> Doc {
    match expr {
        Expr::Integer(lexeme, _) => Doc::Leaf(format!("Integer({lexeme})")),
        Expr::String(lexeme, _) => Doc::Leaf(format!("String({lexeme:?})")),
        Expr::Bool(b, _) => Doc::Leaf(format!("Bool({b})")),
        Expr::Identifier(name, _) => Doc::Leaf(format!("Identifier({name})")),
        Expr::Unary { op, operand, .. } => {
            Doc::Node(format!("Unary({op})"), vec![expr_doc(operand)])
        }
        Expr::Binary {
            op, left, right, ..
        } => Doc::Node(
            format!("Binary({op})"),
            vec![expr_doc(left), expr_doc(right)],
        ),
        Expr::Assign { name, value, .. } => {
            Doc::Node(format!("Assign({name})"), vec![expr_doc(value)])
        }
        Expr::Grouping(inner, _) => Doc::Node("Grouping".to_string(), vec![expr_doc(inner)]),
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
