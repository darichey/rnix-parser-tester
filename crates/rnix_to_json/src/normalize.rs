use crate::ast::{AttrEntry, LambdaArg, NixExpr, StrPart};
use ast::{AttrDef, AttrName, Formal, Formals, NixExpr as NormalNixExpr};
use rnix::{
    types::{BinOpKind, UnaryOpKind},
    value::Anchor,
    NixValue,
};

pub(crate) fn normalize_nix_expr(
    expr: NixExpr,
    base_path: String,
    home_path: String,
) -> NormalNixExpr {
    Normalizer {
        base_path,
        home_path,
    }
    .normalize(expr)
}

struct Normalizer {
    base_path: String,
    home_path: String,
}

impl Normalizer {
    fn normalize(&self, expr: NixExpr) -> NormalNixExpr {
        match expr {
            NixExpr::Apply { lambda, value } => self.normalize_apply(*lambda, *value),
            NixExpr::Assert { condition, body } => self.normalize_assert(*condition, *body),
            NixExpr::Ident(ident) => self.normalize_ident(ident),
            NixExpr::IfElse {
                condition,
                body,
                else_body,
            } => self.normalize_if_else(*condition, *body, *else_body),
            NixExpr::Select { set, index } => self.normalize_select(*set, *index, None),
            NixExpr::Lambda { arg, body } => self.normalize_lambda(arg, *body),
            NixExpr::LetIn { entries, body } => self.normalize_let_in(entries, *body),
            NixExpr::List(elems) => self.normalize_list(elems),
            NixExpr::BinOp { lhs, operator, rhs } => self.normalize_bin_op(*lhs, operator, *rhs),
            NixExpr::OrDefault { index, default } => self.normalize_or_default(*index, *default),
            NixExpr::AttrSet { entries, recursive } => self.normalize_attr_set(entries, recursive),
            NixExpr::Str { parts } => self.normalize_str(parts),
            NixExpr::UnaryOp { operator, value } => self.normalize_unary_op(operator, *value),
            NixExpr::Value(value) => self.normalize_value(value),
            NixExpr::With { namespace, body } => self.normalize_with(*namespace, *body),
        }
    }

    fn boxed_normalize(&self, expr: NixExpr) -> Box<NormalNixExpr> {
        Box::new(self.normalize(expr))
    }

    // Normalize by squashing nested Apply nodes to a single Call node, collecting function arguments into a list
    fn normalize_apply(&self, lambda: NixExpr, value: NixExpr) -> NormalNixExpr {
        let mut fun = lambda;
        let mut args: Vec<NormalNixExpr> = vec![self.normalize(value)];

        while let NixExpr::Apply { lambda, value } = fun {
            args.push(self.normalize(*value));
            fun = *lambda;
        }

        args.reverse();

        NormalNixExpr::Call {
            fun: self.boxed_normalize(fun),
            args,
        }
    }

    fn normalize_assert(&self, condition: NixExpr, body: NixExpr) -> NormalNixExpr {
        NormalNixExpr::Assert {
            cond: self.boxed_normalize(condition),
            body: self.boxed_normalize(body),
        }
    }

    fn normalize_ident(&self, ident: String) -> NormalNixExpr {
        NormalNixExpr::Var(ident)
    }

    fn normalize_if_else(
        &self,
        condition: NixExpr,
        body: NixExpr,
        else_body: NixExpr,
    ) -> NormalNixExpr {
        NormalNixExpr::If {
            cond: self.boxed_normalize(condition),
            then: self.boxed_normalize(body),
            else_: self.boxed_normalize(else_body),
        }
    }

    fn normalize_select(
        &self,
        set: NixExpr,
        index: NixExpr,
        or_default: Option<NixExpr>,
    ) -> NormalNixExpr {
        let mut subject = set;
        let mut path: Vec<AttrName> = vec![self.index_to_atrr_name(index)];

        while let NixExpr::Select { set, index } = subject {
            path.push(self.index_to_atrr_name(*index));
            subject = *set;
        }

        path.reverse();

        NormalNixExpr::Select {
            subject: self.boxed_normalize(subject),
            or_default: or_default.map(|e| self.boxed_normalize(e)),
            path,
        }
    }

    fn index_to_atrr_name(&self, index: NixExpr) -> AttrName {
        match index {
            NixExpr::Ident(ident) => AttrName::Symbol(ident),
            _ => todo!(), // TODO: what else can be here? interpolated keys?
        }
    }

    fn normalize_lambda(&self, arg: LambdaArg, body: NixExpr) -> NormalNixExpr {
        let (arg, formals) = match arg {
            LambdaArg::Ident(ident) => (Some(ident), None),
            LambdaArg::Pattern {
                entries,
                at,
                ellipsis,
            } => {
                let formals = Formals {
                    ellipsis,
                    entries: entries
                        .into_iter()
                        .map(|entry| Formal {
                            name: entry.name,
                            default: entry.default.map(|default| self.normalize(default)),
                        })
                        .collect(),
                };

                (at, Some(formals))
            }
            _ => unreachable!(),
        };

        let body = self.boxed_normalize(body);

        NormalNixExpr::Lambda { arg, formals, body }
    }

    fn normalize_let_in(&self, entries: Vec<AttrEntry>, body: NixExpr) -> NormalNixExpr {
        NormalNixExpr::Let {
            attrs: Box::new(self.normalize_attr_set(entries, false)), // TODO: can let be rec?
            body: self.boxed_normalize(body),
        }
    }

    fn normalize_list(&self, elems: Vec<NixExpr>) -> NormalNixExpr {
        NormalNixExpr::List(elems.into_iter().map(|e| self.normalize(e)).collect())
    }

    fn normalize_bin_op(&self, lhs: NixExpr, operator: BinOpKind, rhs: NixExpr) -> NormalNixExpr {
        match operator {
            BinOpKind::Concat => {
                NormalNixExpr::OpConcatLists(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
            BinOpKind::IsSet => NormalNixExpr::OpHasAttr {
                subject: self.boxed_normalize(lhs),
                path: todo!(),
            },
            BinOpKind::Update => {
                NormalNixExpr::OpUpdate(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
            // The reference parser calls all addition "concat strings"
            BinOpKind::Add => NormalNixExpr::OpConcatStrings {
                force_string: false, // FIXME: I don't know what this is
                es: vec![self.normalize(lhs), self.normalize(rhs)],
            },
            // The reference parser treats subtraction as a call to __sub
            BinOpKind::Sub => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__sub".to_string())),
                args: vec![self.normalize(lhs), self.normalize(rhs)],
            },
            // The reference parser treats multiplication as a call to __mul
            BinOpKind::Mul => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__mul".to_string())),
                args: vec![self.normalize(lhs), self.normalize(rhs)],
            },
            // The reference parser treats division as a call to __div
            BinOpKind::Div => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__div".to_string())),
                args: vec![self.normalize(lhs), self.normalize(rhs)],
            },
            BinOpKind::And => {
                NormalNixExpr::OpAnd(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
            BinOpKind::Equal => {
                NormalNixExpr::OpEq(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
            BinOpKind::Implication => {
                NormalNixExpr::OpImpl(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
            // The reference parser treats less than as a call to __lessThan
            BinOpKind::Less => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
                args: vec![self.normalize(lhs), self.normalize(rhs)],
            },
            // The reference parser treats leq as negating a call to __lessThan with the args flipped
            BinOpKind::LessOrEq => NormalNixExpr::OpNot(Box::new(NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
                // Note the argument order!
                args: vec![self.normalize(rhs), self.normalize(lhs)],
            })),
            // The reference parser treats greater than as a call to __lessThan with the args flipped
            BinOpKind::More => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
                // Note the argument order!
                args: vec![self.normalize(rhs), self.normalize(lhs)],
            },
            // The reference parser treats gte as negating a call to __lessThan
            BinOpKind::MoreOrEq => NormalNixExpr::OpNot(Box::new(NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__lessThan".to_string())),
                args: vec![self.normalize(lhs), self.normalize(rhs)],
            })),
            BinOpKind::NotEqual => {
                NormalNixExpr::OpNEq(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
            BinOpKind::Or => {
                NormalNixExpr::OpOr(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
        }
    }

    // The reference parser merges the Select and OrDefault nodes
    fn normalize_or_default(&self, index: NixExpr, default: NixExpr) -> NormalNixExpr {
        // FIXME: kinda sucks
        match index {
            NixExpr::Select { set, index } => self.normalize_select(*set, *index, Some(default)),
            _ => unreachable!(),
        }
    }

    fn normalize_attr_set(&self, entries: Vec<AttrEntry>, recursive: bool) -> NormalNixExpr {
        let attrs = entries
            .into_iter()
            .flat_map(|entry| match entry {
                AttrEntry::KeyValue { key, value } => {
                    vec![self.normalize_key_value_entry(key, *value)]
                }
                AttrEntry::Inherit { from, idents } => self.normalize_inherit_entry(from, idents),
            })
            .collect();

        NormalNixExpr::Attrs {
            rec: recursive,
            attrs,
            dynamic_attrs: vec![], // TODO: dynamic attrs
        }
    }

    // Normalizing compound keys (e.g., `x.y.z = "hello"` <=> `x = { y = { z = "hello" }}), because the reference impl does this at the parser level
    fn normalize_key_value_entry(
        &self,
        mut path: Vec<NixExpr>,
        value: NixExpr,
    ) -> AttrDef {
        let mut key = match path.pop() {
            Some(NixExpr::Ident(ident)) => ident,
            Some(_) => todo!(),
            None => unreachable!(),
        };

        let mut value = AttrDef {
            name: key,
            inherited: false,
            expr: self.normalize(value),
        };

        while let Some(path_component) = path.pop() {
            key = match path_component {
                NixExpr::Ident(ident) => ident,
                _ => todo!(),
            };

            value = AttrDef {
                name: key,
                inherited: false,
                // TODO: is it possible to have inherited, dynamic attrs, or rec in this case?
                expr: NormalNixExpr::Attrs {
                    rec: false,
                    attrs: vec![value],
                    dynamic_attrs: vec![],
                },
            };
        }

        value
    }

    fn normalize_inherit_entry(
        &self,
        from: Option<Box<NixExpr>>,
        idents: Vec<String>,
    ) -> Vec<AttrDef> {
        let subject = from.map(|from| self.boxed_normalize(*from));

        idents
            .into_iter()
            .map(|ident| {
                if let Some(subject) = &subject {
                    AttrDef {
                        name: ident.clone(),
                        inherited: false, // TODO: really?
                        expr: NormalNixExpr::Select {
                            subject: subject.clone(),
                            or_default: None,
                            path: vec![AttrName::Symbol(ident.clone())],
                        },
                    }
                } else {
                    AttrDef {
                        name: ident.clone(),
                        inherited: true,
                        expr: NormalNixExpr::Var(ident.clone()),
                    }
                }
            })
            .collect()
    }

    fn normalize_str(&self, parts: Vec<StrPart>) -> NormalNixExpr {
        // If any of the parts are Ast, then this string has interoplations in it
        if parts.iter().any(|part| matches!(part, StrPart::Ast(_))) {
            // The reference impl treats string interpolation as string concatenation with force_string: true
            NormalNixExpr::OpConcatStrings {
                force_string: true,
                es: parts
                    .into_iter()
                    .map(|part| match part {
                        StrPart::Literal(lit) => NormalNixExpr::String(lit),
                        StrPart::Ast(expr) => self.normalize(expr),
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

    fn normalize_unary_op(&self, operator: UnaryOpKind, value: NixExpr) -> NormalNixExpr {
        match operator {
            UnaryOpKind::Invert => NormalNixExpr::OpNot(self.boxed_normalize(value)),
            // The reference parser treats negation as subtraction from 0
            UnaryOpKind::Negate => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__sub".to_string())),
                args: vec![NormalNixExpr::Int(0), self.normalize(value)],
            },
        }
    }

    fn normalize_value(&self, value: NixValue) -> NormalNixExpr {
        match value {
            NixValue::Float(nf) => NormalNixExpr::Float(nf),
            NixValue::Integer(n) => NormalNixExpr::Int(n),
            NixValue::String(s) => NormalNixExpr::String(s),
            NixValue::Path(anchor, s) => match anchor {
                Anchor::Absolute => NormalNixExpr::Path(s),
                Anchor::Relative => NormalNixExpr::Path(format!(
                    "{}{}",
                    self.base_path,
                    s.strip_prefix("./.").or(s.strip_prefix("./")).unwrap_or(&s)
                )),
                Anchor::Home => NormalNixExpr::Path(format!("{}/{}", self.home_path, s)),
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

    fn normalize_with(&self, namespace: NixExpr, body: NixExpr) -> NormalNixExpr {
        NormalNixExpr::With {
            attrs: self.boxed_normalize(namespace),
            body: self.boxed_normalize(body),
        }
    }
}
