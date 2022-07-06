mod ast;
mod normalize;

use core::panic;
use std::{fs, path::PathBuf};

use rnix::{
    types::{
        Apply, AttrSet, BinOpKind, EntryHolder, Inherit, KeyValue, Lambda, ParsedType, Select,
        TokenWrapper, UnaryOpKind, Wrapper,
    },
    value::Anchor,
    NixValue, StrPart, SyntaxNode,
};
use serde_json::json;

pub struct Parser {
    base_path: String,
    home_path: String,
}

impl Parser {
    // FIXME: ugly, error handling
    pub fn new(base_path: &str, home_path: String) -> Self {
        let base_path = PathBuf::from(base_path);
        Self {
            base_path: fs::canonicalize(&base_path)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            home_path,
        }
    }

    pub fn parse(&self, nix_expr: &str) -> String {
        let nix_expr = rnix::parse(nix_expr).root().inner();
        self.parsed_type_to_json(nix_expr).to_string()

        // let nix_expr = rnix::parse(nix_expr).root();
        // nix_expr.dump().to_string()
    }

    fn string_parts_to_json(&self, parts: Vec<StrPart>) -> serde_json::Value {
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
                    StrPart::Ast(node) => {
                        self.parsed_type_to_json(node.inner())
                    },
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
    fn select_to_json(&self, mut select: Select) -> serde_json::Value {
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
            "subject": self.parsed_type_to_json(select.set()),
            "or_default": null,
            "path": path.into_iter().rev().collect::<serde_json::Value>(),
        })
    }

    fn lambda_to_json(&self, lambda: Lambda) -> serde_json::Value {
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
                            "default": self.parsed_type_to_json(entry.default()),
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
            "body": self.parsed_type_to_json(lambda.body()),
        })
    }

    fn attr_set_parts_to_json(
        &self,
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

    fn attr_set_to_json(&self, attr_set: AttrSet) -> serde_json::Value {
        self.entry_holder_to_json(&attr_set, attr_set.recursive())
    }

    // Also implements normalizing compound keys (e.g., `x.y.z = "hello"` <=> `x = { y = { z = "hello" }}), because the reference impl does this at the parser level
    fn attr_set_entry_to_json(&self, entry: KeyValue) -> (String, serde_json::Value) {
        let mut path = entry.key().unwrap().path().collect::<Vec<SyntaxNode>>();

        let mut key = match ParsedType::try_from(path.pop().unwrap()).unwrap() {
            ParsedType::Ident(ident) => ident.as_str().to_string(),
            _ => todo!(),
        };

        let mut value = json!({
            "e": self.parsed_type_to_json(entry.value()),
            "inherited": false,
        });

        while let Some(node) = path.pop() {
            value = json!({
                // TODO: is it possible to have inherited, dynamic attrs, or rec in this case?
                "e": self.attr_set_parts_to_json(json!({ key: value }), json!([]), false),
                "inherited": false,
            });

            key = match ParsedType::try_from(node).unwrap() {
                ParsedType::Ident(ident) => ident.as_str().to_string(),
                _ => todo!(),
            };
        }

        (key, value)
    }

    fn inherit_to_json(&self, inherit: Inherit) -> Vec<(String, serde_json::Value)> {
        inherit
            .idents()
            .map(|ident| {
                let value = if let Some(from) = inherit.from() {
                    json!({
                        "e": {
                            "type": "Select",
                            "subject": self.parsed_type_to_json(from.inner()),
                            "or_default": null,
                            "path": [
                                {
                                    "attr_type": "Symbol",
                                    "attr": ident.as_str(),
                                },
                            ],
                        },
                        "inherited": false, // TODO: really?
                    })
                } else {
                    json!({
                        "e": {
                            "type": "Var",
                            "value": ident.as_str(),
                        },
                        "inherited": true,
                    })
                };

                (ident.as_str().to_string(), value)
            })
            .collect()
        // }
    }

    fn entry_holder_to_json(
        &self,
        entry_holder: &impl EntryHolder,
        rec: bool,
    ) -> serde_json::Value {
        let attrs = entry_holder
            .node()
            .children()
            .flat_map(|child| match ParsedType::try_from(child).unwrap() {
                ParsedType::KeyValue(entry) => vec![self.attr_set_entry_to_json(entry)],
                ParsedType::Inherit(inherit) => self.inherit_to_json(inherit),
                _ => unreachable!(),
            })
            .collect();

        // TODO: dynamic attrs
        self.attr_set_parts_to_json(attrs, json!([]), rec)
    }

    fn apply_to_json(&self, apply: Apply) -> serde_json::Value {
        // The reference impl squashes nested Call nodes, collecting function arguments into a single list
        let mut args: Vec<serde_json::Value> = vec![self.parsed_type_to_json(apply.value())];
        let mut value = apply.lambda().unwrap();

        while let ParsedType::Apply(nested_apply) = ParsedType::try_from(value.clone()).unwrap() {
            args.push(self.parsed_type_to_json(nested_apply.value()));
            value = nested_apply.lambda().unwrap();
        }

        args.reverse();

        json!({
            "type": "Call",
            "fun": self.parsed_type_to_json(Some(value)),
            "args": args,
        })
    }

    fn parsed_type_to_json(&self, nix_expr: Option<SyntaxNode>) -> serde_json::Value {
        if nix_expr.is_none() {
            return json!(null);
        }

        let nix_expr = ParsedType::try_from(nix_expr.unwrap()).unwrap();
        match nix_expr {
            ParsedType::Apply(apply) => self.apply_to_json(apply),
            ParsedType::Assert(assert) => json!({
                "type": "Assert",
                "cond": self.parsed_type_to_json(assert.condition()),
                "body": self.parsed_type_to_json(assert.body()),
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
                "cond": self.parsed_type_to_json(if_else.condition()),
                "then": self.parsed_type_to_json(if_else.body()),
                "else": self.parsed_type_to_json(if_else.else_body()),
            }),
            ParsedType::Select(select) => self.select_to_json(select),
            ParsedType::Inherit(_) => todo!(),
            ParsedType::InheritFrom(_) => todo!(),
            ParsedType::Lambda(lambda) => self.lambda_to_json(lambda),
            ParsedType::LegacyLet(_) => todo!(),
            ParsedType::LetIn(let_in) => json!({
                "type": "Let",
                "attrs": self.entry_holder_to_json(&let_in, false), // TODO: can let be rec?
                "body": self.parsed_type_to_json(let_in.body()),
            }),
            ParsedType::List(list) => json!({
                "type": "List",
                "elems": list.items().map(|node| self.parsed_type_to_json(Some(node))).collect::<serde_json::Value>(),
            }),
            ParsedType::BinOp(bin_op) => match bin_op.operator().unwrap() {
                BinOpKind::Concat => json!({
                    "type": "OpConcatLists",
                    "e1": self.parsed_type_to_json(bin_op.lhs()),
                    "e2": self.parsed_type_to_json(bin_op.rhs()),
                }),
                BinOpKind::IsSet => json!({
                    "type": "OpHasAttr",
                    "subject": self.parsed_type_to_json(bin_op.lhs()),
                    "path": self.parsed_type_to_json(bin_op.rhs()),
                }),
                BinOpKind::Update => json!({
                    "type": "OpUpdate",
                    "e1": self.parsed_type_to_json(bin_op.lhs()),
                    "e2": self.parsed_type_to_json(bin_op.rhs()),
                }),
                // The reference parser calls all addition "concat strings"
                BinOpKind::Add => json!({
                    "type": "ConcatStrings",
                    "force_string": false, // FIXME: I don't know what this is
                    "es": [
                        self.parsed_type_to_json(bin_op.lhs()),
                        self.parsed_type_to_json(bin_op.rhs()),
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
                        self.parsed_type_to_json(bin_op.lhs()),
                        self.parsed_type_to_json(bin_op.rhs()),
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
                        self.parsed_type_to_json(bin_op.lhs()),
                        self.parsed_type_to_json(bin_op.rhs()),
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
                        self.parsed_type_to_json(bin_op.lhs()),
                        self.parsed_type_to_json(bin_op.rhs()),
                    ]
                }),
                BinOpKind::And => json!({
                    "type": "OpAnd",
                    "e1": self.parsed_type_to_json(bin_op.lhs()),
                    "e2": self.parsed_type_to_json(bin_op.rhs()),
                }),
                BinOpKind::Equal => json!({
                    "type": "OpEq",
                    "e1": self.parsed_type_to_json(bin_op.lhs()),
                    "e2": self.parsed_type_to_json(bin_op.rhs()),
                }),
                BinOpKind::Implication => json!({
                    "type": "OpImpl",
                    "e1": self.parsed_type_to_json(bin_op.lhs()),
                    "e2": self.parsed_type_to_json(bin_op.rhs()),
                }),
                // The reference parser treats less than as a call to __lessThan
                BinOpKind::Less => json!({
                    "type": "Call",
                    "fun": {
                        "type": "Var",
                        "value": "__lessThan",
                    },
                    "args": [
                        self.parsed_type_to_json(bin_op.lhs()),
                        self.parsed_type_to_json(bin_op.rhs()),
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
                            self.parsed_type_to_json(bin_op.rhs()),
                            self.parsed_type_to_json(bin_op.lhs()),
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
                        self.parsed_type_to_json(bin_op.rhs()),
                        self.parsed_type_to_json(bin_op.lhs()),
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
                            self.parsed_type_to_json(bin_op.lhs()),
                            self.parsed_type_to_json(bin_op.rhs()),
                        ],
                    },
                }),
                BinOpKind::NotEqual => json!({
                    "type": "OpNEq",
                    "e1": self.parsed_type_to_json(bin_op.lhs()),
                    "e2": self.parsed_type_to_json(bin_op.rhs()),
                }),
                BinOpKind::Or => json!({
                    "type": "OpOr",
                    "e1": self.parsed_type_to_json(bin_op.lhs()),
                    "e2": self.parsed_type_to_json(bin_op.rhs()),
                }),
            },
            // The reference parser merges the Select and OrDefault nodes
            ParsedType::OrDefault(or_default) => {
                let mut select = self.select_to_json(or_default.index().unwrap());
                select["or_default"] = self.parsed_type_to_json(or_default.default());
                select
            },
            ParsedType::Paren(paren) => self.parsed_type_to_json(paren.inner()),
            ParsedType::PatBind(_) => todo!(),
            ParsedType::PatEntry(_) => todo!(),
            ParsedType::Pattern(_) => todo!(),
            ParsedType::Root(root) => self.parsed_type_to_json(root.inner()),
            ParsedType::AttrSet(attr_set) => self.attr_set_to_json(attr_set),
            ParsedType::KeyValue(_) => todo!(),
            ParsedType::Str(str) => self.string_parts_to_json(str.parts()),
            ParsedType::StrInterpol(_) => todo!(),
            ParsedType::UnaryOp(unary_op) => match unary_op.operator().unwrap() {
                UnaryOpKind::Invert => json!({
                    "type": "OpNot",
                    "e": self.parsed_type_to_json(unary_op.value()),
                }),
                // The reference parser treats negation as subtraction from 0
                UnaryOpKind::Negate => json!({
                    "type": "Call",
                    "args": [
                        {
                            "type": "Int",
                            "value": 0,
                        },
                        self.parsed_type_to_json(unary_op.value()),
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
                NixValue::Path(anchor, s) => match anchor {
                    Anchor::Absolute => json!({
                        "type": "Path",
                        "value": s,
                    }),
                    Anchor::Relative => json!({
                        "type": "Path",
                        "value": format!("{}{}", self.base_path, s.strip_prefix("./.").or(s.strip_prefix("./")).unwrap_or(&s)),
                    }),
                    Anchor::Home => json!({
                        "type": "Path",
                        "value": format!("{}/{}", self.home_path, s),
                    }),
                    // The reference impl treats store paths as a call to __findFile with the args __nixPath and the path
                    Anchor::Store => json!({
                        "type": "Call",
                        "fun": {
                            "type": "Var",
                            "value": "__findFile",
                        },
                        "args": [
                            {
                                "type": "Var",
                                "value": "__nixPath",
                            },
                            {
                                "type": "String",
                                "value": s
                            },
                        ],
                    }),
                },
            },
            ParsedType::With(with) => json!({
                "type": "With",
                "attrs": self.parsed_type_to_json(with.namespace()),
                "body": self.parsed_type_to_json(with.body()),
            }),
            ParsedType::PathWithInterpol(_) => todo!(),
        }
    }
}
