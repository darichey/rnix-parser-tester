use std::collections::HashMap;

use crate::ast::{AttrEntry, NixExpr, StrPart};
use ast::{AttrDef, AttrName, AttrPath, NixExpr as NormalNixExpr};
use rnix::{
    types::{BinOpKind, UnaryOpKind},
    value::Anchor,
    NixValue,
};

pub(crate) fn normalize_nix_expr(expr: NixExpr) -> NormalNixExpr {
    match expr {
        NixExpr::Apply { lambda, value } => normalize_apply(*lambda, *value),
        NixExpr::Assert { condition, body } => normalize_assert(*condition, *body),
        NixExpr::Ident(ident) => normalize_ident(ident),
        NixExpr::IfElse {
            condition,
            body,
            else_body,
        } => normalize_if_else(*condition, *body, *else_body),
        NixExpr::Select { set, index } => normalize_select(*set, *index, None),
        NixExpr::Lambda { arg, body } => normalize_lambda(*arg, *body),
        NixExpr::LetIn { entries, body } => normalize_let_in(entries, *body),
        NixExpr::List(elems) => normalize_list(elems),
        NixExpr::BinOp { lhs, operator, rhs } => normalize_bin_op(*lhs, operator, *rhs),
        NixExpr::OrDefault { index, default } => normalize_or_default(*index, *default),
        NixExpr::AttrSet { entries, recursive } => normalize_attr_set(entries, recursive),
        NixExpr::Str { parts } => normalize_str(parts),
        NixExpr::UnaryOp { operator, value } => normalize_unary_op(operator, *value),
        NixExpr::Value(value) => normalize_value(value),
        NixExpr::With { namespace, body } => normalize_with(*namespace, *body),
    }
}

fn normalize_with(namespace: NixExpr, body: NixExpr) -> NormalNixExpr {
    NormalNixExpr::With {
        attrs: Box::new(normalize_nix_expr(namespace)),
        body: Box::new(normalize_nix_expr(body)),
    }
}

fn normalize_value(value: NixValue) -> NormalNixExpr {
    match value {
        NixValue::Float(nf) => NormalNixExpr::Float(nf),
        NixValue::Integer(n) => NormalNixExpr::Int(n),
        NixValue::String(s) => NormalNixExpr::String(s),
        NixValue::Path(anchor, s) => match anchor {
            Anchor::Absolute => NormalNixExpr::Path(s),
            Anchor::Relative => NormalNixExpr::Path(format!(
                "{}{}",
                todo!("base_path"),
                s.strip_prefix("./.").or(s.strip_prefix("./")).unwrap_or(&s)
            )),
            Anchor::Home => NormalNixExpr::Path(format!("{}/{}", todo!("home_path"), s)),
            // The reference impl treats store paths as a call to __findFile with the args __nixPath and the path
            Anchor::Store => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__findFile".to_string())),
                args: vec![
                    NormalNixExpr::Var("__nixPath".to_string()),
                    NormalNixExpr::String(s),
                ],
            },
        },
    }
}

fn normalize_unary_op(operator: UnaryOpKind, value: NixExpr) -> NormalNixExpr {
    match operator {
        UnaryOpKind::Invert => NormalNixExpr::OpNot(Box::new(normalize_nix_expr(value))),
        // The reference parser treats negation as subtraction from 0
        UnaryOpKind::Negate => NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__sub".to_string())),
            args: vec![NormalNixExpr::Int(0), normalize_nix_expr(value)],
        },
    }
}

fn normalize_str(parts: Vec<StrPart>) -> NormalNixExpr {
    // If any of the parts are Ast, then this string has interoplations in it
    if parts.iter().any(|part| matches!(part, StrPart::Ast(_))) {
        // The reference impl treats string interpolation as string concatenation with force_string: true
        NormalNixExpr::OpConcatStrings {
            force_string: true,
            es: parts
                .into_iter()
                .map(|part| match part {
                    StrPart::Literal(lit) => NormalNixExpr::String(lit),
                    StrPart::Ast(expr) => normalize_nix_expr(expr),
                })
                .collect(),
        }
    // otherwise, there should only be one part which is a literal
    } else if let Some(StrPart::Literal(lit)) = parts.get(0) {
        NormalNixExpr::String(lit.to_string())
    } else {
        unreachable!()
    }
}

fn normalize_attr_set(entries: Vec<AttrEntry>, recursive: bool) -> NormalNixExpr {
    let attrs = entries
        .into_iter()
        .flat_map(|entry| match entry {
            AttrEntry::KeyValue { key, value } => vec![normalize_key_value_entry(key, *value)],
            AttrEntry::Inherit { from, idents } => normalize_inherit_entry(from, idents),
        })
        .collect();

    NormalNixExpr::Attrs {
        rec: recursive,
        attrs,
    }
}

fn normalize_inherit_entry(
    from: Option<Box<NixExpr>>,
    idents: Vec<String>,
) -> Vec<(String, AttrDef)> {
    let subject = from.map(|from| Box::new(normalize_nix_expr(*from)));

    idents
        .into_iter()
        .map(|ident| {
            let value = if let Some(subject) = &subject {
                AttrDef {
                    inherited: false, // TODO: really?
                    expr: NormalNixExpr::Select {
                        subject: subject.clone(),
                        or_default: None,
                        path: AttrPath {
                            components: vec![AttrName::Symbol(ident.clone())],
                        },
                    },
                }
            } else {
                AttrDef {
                    inherited: true,
                    expr: NormalNixExpr::Var(ident.clone()),
                }
            };

            (ident, value)
        })
        .collect()
}

// Normalizing compound keys (e.g., `x.y.z = "hello"` <=> `x = { y = { z = "hello" }}), because the reference impl does this at the parser level
fn normalize_key_value_entry(mut path: Vec<NixExpr>, value: NixExpr) -> (String, AttrDef) {
    let mut key = match path.pop() {
        Some(NixExpr::Ident(ident)) => ident,
        Some(_) => todo!(),
        None => unreachable!(),
    };

    let mut value = AttrDef {
        inherited: false,
        expr: normalize_nix_expr(value),
    };

    while let Some(path_component) = path.pop() {
        value = AttrDef {
            inherited: false,
            // TODO: is it possible to have inherited, dynamic attrs, or rec in this case?
            expr: NormalNixExpr::Attrs {
                rec: false,
                attrs: HashMap::from([(key, value)]),
            },
        };

        key = match path_component {
            NixExpr::Ident(ident) => ident,
            _ => todo!(),
        };
    }

    (key, value)
}

// The reference parser merges the Select and OrDefault nodes
fn normalize_or_default(index: NixExpr, default: NixExpr) -> NormalNixExpr {
    // FIXME: kinda sucks
    match index {
        NixExpr::Select { set, index } => normalize_select(*set, *index, Some(default)),
        _ => unreachable!(),
    }
}

fn normalize_bin_op(lhs: NixExpr, operator: BinOpKind, rhs: NixExpr) -> NormalNixExpr {
    match operator {
        BinOpKind::Concat => NormalNixExpr::OpConcatLists(
            Box::new(normalize_nix_expr(lhs)),
            Box::new(normalize_nix_expr(rhs)),
        ),
        BinOpKind::IsSet => NormalNixExpr::OpHasAttr {
            subject: Box::new(normalize_nix_expr(lhs)),
            path: AttrPath {
                components: todo!(),
            },
        },
        BinOpKind::Update => NormalNixExpr::OpUpdate(
            Box::new(normalize_nix_expr(lhs)),
            Box::new(normalize_nix_expr(rhs)),
        ),
        // The reference parser calls all addition "concat strings"
        BinOpKind::Add => NormalNixExpr::OpConcatStrings {
            force_string: false, // FIXME: I don't know what this is
            es: vec![normalize_nix_expr(lhs), normalize_nix_expr(rhs)],
        },
        // The reference parser treats subtraction as a call to __sub
        BinOpKind::Sub => NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__sub".to_string())),
            args: vec![normalize_nix_expr(lhs), normalize_nix_expr(rhs)],
        },
        // The reference parser treats multiplication as a call to __mul
        BinOpKind::Mul => NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__mul".to_string())),
            args: vec![normalize_nix_expr(lhs), normalize_nix_expr(rhs)],
        },
        // The reference parser treats division as a call to __div
        BinOpKind::Div => NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__div".to_string())),
            args: vec![normalize_nix_expr(lhs), normalize_nix_expr(rhs)],
        },
        BinOpKind::And => NormalNixExpr::OpAnd(
            Box::new(normalize_nix_expr(lhs)),
            Box::new(normalize_nix_expr(rhs)),
        ),
        BinOpKind::Equal => NormalNixExpr::OpEq(
            Box::new(normalize_nix_expr(lhs)),
            Box::new(normalize_nix_expr(rhs)),
        ),
        BinOpKind::Implication => NormalNixExpr::OpImpl(
            Box::new(normalize_nix_expr(lhs)),
            Box::new(normalize_nix_expr(rhs)),
        ),
        // The reference parser treats less than as a call to __lessThan
        BinOpKind::Less => NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
            args: vec![normalize_nix_expr(lhs), normalize_nix_expr(rhs)],
        },
        // The reference parser treats leq as negating a call to __lessThan with the args flipped
        BinOpKind::LessOrEq => NormalNixExpr::OpNot(Box::new(NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
            args: vec![normalize_nix_expr(lhs), normalize_nix_expr(rhs)],
        })),
        // The reference parser treats greater than as a call to __lessThan with the args flipped
        BinOpKind::More => NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
            // Note the argument order!
            args: vec![normalize_nix_expr(rhs), normalize_nix_expr(lhs)],
        },
        // The reference parser treats gte as negating a call to __lessThan
        BinOpKind::MoreOrEq => NormalNixExpr::OpNot(Box::new(NormalNixExpr::Call {
            fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
            // Note the argument order!
            args: vec![normalize_nix_expr(rhs), normalize_nix_expr(lhs)],
        })),
        BinOpKind::NotEqual => NormalNixExpr::OpNEq(
            Box::new(normalize_nix_expr(lhs)),
            Box::new(normalize_nix_expr(rhs)),
        ),
        BinOpKind::Or => NormalNixExpr::OpOr(
            Box::new(normalize_nix_expr(lhs)),
            Box::new(normalize_nix_expr(rhs)),
        ),
    }
}

fn normalize_list(elems: Vec<NixExpr>) -> NormalNixExpr {
    NormalNixExpr::List(elems.into_iter().map(normalize_nix_expr).collect())
}

fn normalize_let_in(entries: Vec<AttrEntry>, body: NixExpr) -> NormalNixExpr {
    let attrs = todo!();
    let body = Box::new(normalize_nix_expr(body));
    NormalNixExpr::Let { attrs, body }
}

fn normalize_lambda(arg: NixExpr, body: NixExpr) -> NormalNixExpr {
    let (arg, formals) = match arg {
        NixExpr::Ident(ident) => (Some(ident), None),
        _ => todo!(), // TODO pattern
    };

    let body = Box::new(normalize_nix_expr(body));

    NormalNixExpr::Lambda { arg, formals, body }
}

fn index_to_atrr_name(index: NixExpr) -> AttrName {
    match index {
        NixExpr::Ident(ident) => AttrName::Symbol(ident),
        _ => todo!(), // TODO: what else can be here? interpolated keys?
    }
}

fn normalize_select(set: NixExpr, index: NixExpr, or_default: Option<NixExpr>) -> NormalNixExpr {
    let mut subject = set;
    let mut components: Vec<AttrName> = vec![index_to_atrr_name(index)];

    while let NixExpr::Select { set, index } = subject {
        components.push(index_to_atrr_name(*index));
        subject = *set;
    }

    NormalNixExpr::Select {
        subject: Box::new(normalize_nix_expr(subject)),
        or_default: or_default.map(|e| Box::new(normalize_nix_expr(e))),
        path: AttrPath { components },
    }
}

fn normalize_if_else(condition: NixExpr, body: NixExpr, else_body: NixExpr) -> NormalNixExpr {
    NormalNixExpr::If {
        cond: Box::new(normalize_nix_expr(condition)),
        then: Box::new(normalize_nix_expr(body)),
        else_: Box::new(normalize_nix_expr(else_body)),
    }
}

fn normalize_ident(ident: String) -> NormalNixExpr {
    NormalNixExpr::Var(ident)
}

fn normalize_assert(condition: NixExpr, body: NixExpr) -> NormalNixExpr {
    NormalNixExpr::Assert {
        cond: Box::new(normalize_nix_expr(condition)),
        body: Box::new(normalize_nix_expr(body)),
    }
}

// Normalize by squashing nested Apply nodes to a single Call node, collecting function arguments into a list
fn normalize_apply(lambda: NixExpr, value: NixExpr) -> NormalNixExpr {
    let mut fun = lambda;
    let mut args: Vec<NormalNixExpr> = vec![normalize_nix_expr(value)];

    while let NixExpr::Apply { lambda, value } = fun {
        args.push(normalize_nix_expr(*value));
        fun = *lambda;
    }

    args.reverse();

    NormalNixExpr::Call {
        fun: Box::new(normalize_nix_expr(fun)),
        args,
    }
}
