use core::panic;

use rnix::{
    types::{
        AttrSet, BinOpKind, EntryHolder, KeyValue, Lambda, ParsedType, Select, TokenWrapper,
        TypedNode, UnaryOpKind, Wrapper,
    },
    NixValue, StrPart, SyntaxNode, WalkEvent,
};
use serde_json::json;

pub fn nix_expr_to_json(nix_expr: &str) -> String {
    let nix_expr = rnix::parse(nix_expr).root().inner();
    parsed_type_to_json(nix_expr).to_string()

    // let nix_expr = rnix::parse(nix_expr).root();
    // nix_expr.dump().to_string()
}

fn string_parts_to_json(parts: Vec<StrPart>) -> serde_json::Value {
    // If any of the parts are Ast, then this string has interoplations in it
    if parts.iter().any(|part| matches!(part, StrPart::Ast(_))) {
        // The reference impl treats string interpolation as string concatenation with force_string: true
        json!({
            "type": "ConcatStrings",
            "force_string": true,
            "es": parts.into_iter().map(|part| match part {
                StrPart::Literal(lit) => json!({
                    "type": "String",
                    "value": lit,
                }),
                StrPart::Ast(node) => parsed_type_to_json(Some(node)),
            }).collect::<serde_json::Value>()
        })
    // otherwise, there should only be one part which is a literal
    } else if let Some(StrPart::Literal(lit)) = parts.get(0) {
        json!({
            "type": "String",
            "value": lit,
        })
    } else {
        unreachable!()
    }
}

// FIXME: this is the worst thing I have ever seen
// The reference impl squashes nested select nodes
fn select_to_json(mut select: Select) -> serde_json::Value {
    let mut path: Vec<serde_json::Value> = vec![];

    let index = ParsedType::try_from(select.index().unwrap()).unwrap();
    if let ParsedType::Ident(ident) = index {
        path.push(json!({
            "attr_type": "Symbol",
            "attr": ident.as_str(),
        }));
    } else {
        // TODO: interpolated keys
        todo!();
    }

    let mut set = select.set().unwrap();
    while let ParsedType::Select(nested_select) = ParsedType::try_from(set).unwrap() {
        let index = ParsedType::try_from(nested_select.index().unwrap()).unwrap();
        if let ParsedType::Ident(ident) = index {
            path.push(json!({
                "attr_type": "Symbol",
                "attr": ident.as_str(),
            }));
        } else {
            // TODO: interpolated keys
            todo!();
        }
        set = nested_select.set().unwrap();
        select = nested_select;
    }

    json!({
        "type": "Select",
        "subject": parsed_type_to_json(select.set()),
        "or_default": null,
        "path": path.into_iter().rev().collect::<serde_json::Value>(),
    })
}

fn lambda_to_json(lambda: Lambda) -> serde_json::Value {
    let arg_node = ParsedType::try_from(lambda.arg().unwrap()).unwrap();

    let (arg, formals): (Option<String>, Option<serde_json::Value>) = match arg_node {
        ParsedType::Ident(ident) => (Some(ident.as_str().to_string()), None),
        ParsedType::Pattern(pattern) => {
            let arg = pattern.at().map(|ident| ident.as_str().to_string());
            let formals = json!({
                "ellipsis": pattern.ellipsis(),
                "entries": pattern.entries().map(|entry| {
                    json!({
                        "name": entry.name().unwrap().as_str(),
                        "default": parsed_type_to_json(entry.default()),
                    })
                }).collect::<serde_json::Value>(),
            });
            (arg, Some(formals))
        }
        _ => unreachable!(),
    };

    json!({
        "type": "Lambda",
        "arg": arg,
        "formals": formals,
        "body": parsed_type_to_json(lambda.body()),
    })
}

fn attr_set_parts_to_json(
    attrs: serde_json::Value,
    dynamic_attrs: serde_json::Value,
    rec: bool,
) -> serde_json::Value {
    json!({
        "type": "Attrs",
        "attrs": attrs,
        "dynamic_attrs": dynamic_attrs,
        "rec": rec,
    })
}

fn attr_set_to_json(attr_set: AttrSet) -> serde_json::Value {
    entry_holder_to_json(&attr_set, attr_set.recursive())
}

// Also implements normalizing compound keys (e.g., `x.y.z = "hello"` <=> `x = { y = { z = "hello" }}), because the reference impl does this at the parser level
fn attr_set_entry_to_json(entry: KeyValue) -> (String, serde_json::Value) {
    let mut path = entry.key().unwrap().path().collect::<Vec<SyntaxNode>>();

    let mut key = match ParsedType::try_from(path.pop().unwrap()).unwrap() {
        ParsedType::Ident(ident) => ident.as_str().to_string(),
        _ => todo!(),
    };

    let mut value = json!({
        "e": parsed_type_to_json(entry.value()),
        "inherited": false,
    });

    while let Some(node) = path.pop() {
        value = json!({
            // TODO: is it possible to have inherited, dynamic attrs, or rec in this case?
            "e": attr_set_parts_to_json(json!({ key: value }), json!([]), false),
            "inherited": false,
        });

        key = match ParsedType::try_from(node).unwrap() {
            ParsedType::Ident(ident) => ident.as_str().to_string(),
            _ => todo!(),
        };
    }

    (key, value)
}

fn entry_holder_to_json(entry_holder: &impl EntryHolder, rec: bool) -> serde_json::Value {
    let attrs = entry_holder
        .entries()
        .map(attr_set_entry_to_json)
        .collect::<serde_json::Value>();

    // TODO: dynamic attrs
    attr_set_parts_to_json(attrs, json!([]), rec)
}

fn parsed_type_to_json(nix_expr: Option<SyntaxNode>) -> serde_json::Value {
    if nix_expr.is_none() {
        return json!(null);
    }

    let nix_expr = ParsedType::try_from(nix_expr.unwrap()).unwrap();
    match nix_expr {
        ParsedType::Apply(apply) => json!({
            "type": "Call",
            "fun": parsed_type_to_json(apply.lambda()),
            "args": [ parsed_type_to_json(apply.value()) ], // FIXME: the reference parser collects all function arguments into a single Call node
        }),
        ParsedType::Assert(assert) => json!({
            "type": "Assert",
            "cond": parsed_type_to_json(assert.condition()),
            "body": parsed_type_to_json(assert.body()),
        }),
        ParsedType::Key(_) => todo!(),
        ParsedType::Dynamic(_) => todo!(),
        ParsedType::Error(_) => panic!("nix_expr_to_json can only be used on well-formed Nix expressions (i.e., parse errors cannot be present)"),
        ParsedType::Ident(ident) => json!({
            "type": "Var",
            "value": ident.as_str(),
        }),
        ParsedType::IfElse(if_else) => json!({
            "type": "If",
            "cond": parsed_type_to_json(if_else.condition()),
            "then": parsed_type_to_json(if_else.body()),
            "else": parsed_type_to_json(if_else.else_body()),
        }),
        ParsedType::Select(select) => select_to_json(select),
        ParsedType::Inherit(_) => todo!(),
        ParsedType::InheritFrom(_) => todo!(),
        ParsedType::Lambda(lambda) => lambda_to_json(lambda),
        ParsedType::LegacyLet(_) => todo!(),
        ParsedType::LetIn(let_in) => json!({
            "type": "Let",
            "attrs": entry_holder_to_json(&let_in, false), // TODO: can let be rec?
            "body": parsed_type_to_json(let_in.body()),
        }),
        ParsedType::List(list) => json!({
            "type": "List",
            "elems": list.items().map(|node| parsed_type_to_json(Some(node))).collect::<serde_json::Value>(),
        }),
        ParsedType::BinOp(bin_op) => match bin_op.operator().unwrap() {
            BinOpKind::Concat => json!({
                "type": "OpConcatLists",
                "e1": parsed_type_to_json(bin_op.lhs()),
                "e2": parsed_type_to_json(bin_op.rhs()),
            }),
            BinOpKind::IsSet => json!({
                "type": "OpHasAttr",
                "subject": parsed_type_to_json(bin_op.lhs()),
                "path": parsed_type_to_json(bin_op.rhs()),
            }),
            BinOpKind::Update => json!({
                "type": "OpUpdate",
                "e1": parsed_type_to_json(bin_op.lhs()),
                "e2": parsed_type_to_json(bin_op.rhs()),
            }),
            // The reference parser calls all addition "concat strings"
            BinOpKind::Add => json!({
                "type": "ConcatStrings",
                "force_string": false, // FIXME: I don't know what this is
                "es": [
                    parsed_type_to_json(bin_op.lhs()),
                    parsed_type_to_json(bin_op.rhs()),
                ],
            }),
            // The reference parser treats subtraction as a call to __sub
            BinOpKind::Sub => json!({
                "type": "Call",
                "fun": {
                    "type": "Var",
                    "value": "__sub",
                },
                "args": [
                    parsed_type_to_json(bin_op.lhs()),
                    parsed_type_to_json(bin_op.rhs()),
                ]
            }),
            // The reference parser treats multiplication as a call to __mul
            BinOpKind::Mul => json!({
                "type": "Call",
                "fun": {
                    "type": "Var",
                    "value": "__mul",
                },
                "args": [
                    parsed_type_to_json(bin_op.lhs()),
                    parsed_type_to_json(bin_op.rhs()),
                ]
            }),
            // The reference parser treats division as a call to __div
            BinOpKind::Div => json!({
                "type": "Call",
                "fun": {
                    "type": "Var",
                    "value": "__div",
                },
                "args": [
                    parsed_type_to_json(bin_op.lhs()),
                    parsed_type_to_json(bin_op.rhs()),
                ]
            }),
            BinOpKind::And => json!({
                "type": "OpAnd",
                "e1": parsed_type_to_json(bin_op.lhs()),
                "e2": parsed_type_to_json(bin_op.rhs()),
            }),
            BinOpKind::Equal => json!({
                "type": "OpEq",
                "e1": parsed_type_to_json(bin_op.lhs()),
                "e2": parsed_type_to_json(bin_op.rhs()),
            }),
            BinOpKind::Implication => json!({
                "type": "OpImpl",
                "e1": parsed_type_to_json(bin_op.lhs()),
                "e2": parsed_type_to_json(bin_op.rhs()),
            }),
            // The reference parser treats less than as a call to __lessThan
            BinOpKind::Less => json!({
                "type": "Call",
                "fun": {
                    "type": "Var",
                    "value": "__lessThan",
                },
                "args": [
                    parsed_type_to_json(bin_op.lhs()),
                    parsed_type_to_json(bin_op.rhs()),
                ]
            }),
            // The reference parser treats leq as negating a call to __lessThan with the args flipped
            BinOpKind::LessOrEq => json!({
                "type": "OpNot",
                "e": {
                    "type": "Call",
                    "fun": {
                        "type": "Var",
                        "value": "__lessThan"
                    },
                    "args": [
                        // Note the argument order!
                        parsed_type_to_json(bin_op.rhs()),
                        parsed_type_to_json(bin_op.lhs()),
                    ],
                },
            }),
            // The reference parser treats greater than as a call to __lessThan with the args flipped
            BinOpKind::More => json!({
                "type": "Call",
                "fun": {
                    "type": "Var",
                    "value": "__lessThan",
                },
                "args": [
                    // Note the argument order!
                    parsed_type_to_json(bin_op.rhs()),
                    parsed_type_to_json(bin_op.lhs()),
                ]
            }),
            // The reference parser treats gte as negating a call to __lessThan
            BinOpKind::MoreOrEq => json!({
                "type": "OpNot",
                "e": {
                    "type": "Call",
                    "fun": {
                        "type": "Var",
                        "value": "__lessThan"
                    },
                    "args": [
                        parsed_type_to_json(bin_op.lhs()),
                        parsed_type_to_json(bin_op.rhs()),
                    ],
                },
            }),
            BinOpKind::NotEqual => json!({
                "type": "OpNEq",
                "e1": parsed_type_to_json(bin_op.lhs()),
                "e2": parsed_type_to_json(bin_op.rhs()),
            }),
            BinOpKind::Or => json!({
                "type": "OpOr",
                "e1": parsed_type_to_json(bin_op.lhs()),
                "e2": parsed_type_to_json(bin_op.rhs()),
            }),
        },
        // The reference parser merges the Select and OrDefault nodes
        ParsedType::OrDefault(or_default) => {
            let mut select = select_to_json(or_default.index().unwrap());
            select["or_default"] = parsed_type_to_json(or_default.default());
            select
        },
        ParsedType::Paren(paren) => parsed_type_to_json(paren.inner()),
        ParsedType::PatBind(_) => todo!(),
        ParsedType::PatEntry(_) => todo!(),
        ParsedType::Pattern(_) => todo!(),
        ParsedType::Root(root) => parsed_type_to_json(root.inner()),
        ParsedType::AttrSet(attr_set) => attr_set_to_json(attr_set),
        ParsedType::KeyValue(_) => todo!(),
        ParsedType::Str(str) => string_parts_to_json(str.parts()),
        ParsedType::UnaryOp(unary_op) => match unary_op.operator() {
            UnaryOpKind::Invert => json!({
                "type": "OpNot",
                "e": parsed_type_to_json(unary_op.value()),
            }),
            // The reference parser treats negation as subtraction from 0
            UnaryOpKind::Negate => json!({
                "type": "Call",
                "args": [
                    {
                        "type": "Int",
                        "value": 0,
                    },
                    parsed_type_to_json(unary_op.value()),
                ],
            }),
        },
        ParsedType::Value(value) => match value.to_value().unwrap() {
            NixValue::Float(nf) => json!({
                "type": "Float",
                "value": nf,
            }),
            NixValue::Integer(n) => json!({
                "type": "Int",
                "value": n,
            }),
            NixValue::String(s) => json!({
                "type": "String",
                "value": s,
            }),
            NixValue::Path(_, _) => todo!(),
        },
        ParsedType::With(with) => json!({
            "type": "With",
            "attrs": parsed_type_to_json(with.namespace()),
            "body": parsed_type_to_json(with.body()),
        }),
        ParsedType::PathWithInterpol(_) => todo!(),
    }
}
