use core::panic;

use rnix::{
    types::{BinOpKind, ParsedType, TokenWrapper, UnaryOpKind, Wrapper},
    NixValue, SyntaxNode,
};
use serde_json::json;

pub fn nix_expr_to_json(nix_expr: &str) -> String {
    let nix_expr = rnix::parse(nix_expr).root().inner();
    parsed_type_to_json(nix_expr).to_string()
}

fn parsed_type_to_json(nix_expr: Option<SyntaxNode>) -> serde_json::Value {
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
        ParsedType::Select(_) => todo!(),
        ParsedType::Inherit(_) => todo!(),
        ParsedType::InheritFrom(_) => todo!(),
        ParsedType::Lambda(_) => todo!(),
        ParsedType::LegacyLet(_) => todo!(),
        ParsedType::LetIn(_) => todo!(),
        ParsedType::List(_) => todo!(),
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
        ParsedType::OrDefault(_) => todo!(),
        ParsedType::Paren(paren) => parsed_type_to_json(paren.inner()),
        ParsedType::PatBind(_) => todo!(),
        ParsedType::PatEntry(_) => todo!(),
        ParsedType::Pattern(_) => todo!(),
        ParsedType::Root(root) => parsed_type_to_json(root.inner()),
        ParsedType::AttrSet(_) => todo!(),
        ParsedType::KeyValue(_) => todo!(),
        ParsedType::Str(_) => todo!(),
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
        ParsedType::With(_) => todo!(),
        ParsedType::PathWithInterpol(_) => todo!(),
    }
}
