use ast::{AttrDef, AttrName, DynamicAttrDef, Formal, Formals, NixExpr as NormalNixExpr};
use rnix_ast::ast::{
    Anchor, Apply, Assert, AttrSet, BinOp, BinOpKind, Entry, Ident, IfElse, Inherit, Key, KeyValue,
    Lambda, LegacyLet, LetIn, List, NixExpr as RNixExpr, NixValue, OrDefault, Paren, Path,
    PathPart, PathWithInterpol, Root, Select, Str, StrPart, UnaryOp, UnaryOpKind, With,
};

pub fn normalize_nix_expr(expr: RNixExpr, base_path: String, home_path: String) -> NormalNixExpr {
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

/// Called to indicate that some assumption we made about the structure of [`RNixExpr`] was violated.
///
/// Not every [`RNixExpr`] variant directly corresponds to a [`NormalNixExpr`] variant.
/// For example, we cannot directly normalize a [`RNixExpr::KeyValue`]. Instead, it will be
/// handled when normalizing its parent [`RNixExpr::AttrSet`]. So, if we ever encounter a
/// [`RNixExpr::KeyValue`] that is _not_ within a [`RNixExpr::AttrSet`], something has gone
/// wrong, so we panic, giving info about what we encountered and how we thought it should
/// have been handled.
macro_rules! unhandled_normalization_path {
    ($encountered:expr, $thought_handled:expr) => {
        panic!("Unhandled normalization path: encountered {:?} but I thought this would be handled by {}", $encountered, $thought_handled)
    };
}

impl Normalizer {
    fn normalize(&self, expr: RNixExpr) -> NormalNixExpr {
        match expr {
            RNixExpr::Apply(apply) => self.normalize_apply(apply),
            RNixExpr::Assert(assert) => self.normalize_assert(assert),
            RNixExpr::Key(key) => unhandled_normalization_path!(key, "attr set normalization"),
            RNixExpr::Dynamic(dynamic) => {
                unhandled_normalization_path!(dynamic, "attr set normalization")
            }
            RNixExpr::Ident(ident) => self.normalize_ident(ident),
            RNixExpr::IfElse(if_else) => self.normalize_if_else(if_else),
            RNixExpr::Select(select) => self.normalize_select(select),
            RNixExpr::Inherit(inherit) => {
                unhandled_normalization_path!(inherit, "attr set normalization")
            }
            RNixExpr::InheritFrom(inherit_from) => {
                unhandled_normalization_path!(inherit_from, "attr set normalization")
            }
            RNixExpr::Lambda(lambda) => self.normalize_lambda(lambda),
            RNixExpr::LegacyLet(legacy_let) => self.normalize_legacy_let(legacy_let),
            RNixExpr::LetIn(let_in) => self.normalize_let_in(let_in),
            RNixExpr::List(list) => self.normalize_list(list),
            RNixExpr::BinOp(bin_op) => self.normalize_bin_op(bin_op),
            RNixExpr::OrDefault(or_default) => self.normalize_or_default(or_default),
            RNixExpr::Paren(paren) => self.normalize_paren(paren),
            RNixExpr::PatBind(pat_bind) => {
                unhandled_normalization_path!(pat_bind, "lambda normalization")
            }
            RNixExpr::PatEntry(pat_entry) => {
                unhandled_normalization_path!(pat_entry, "lambda normalization")
            }
            RNixExpr::Pattern(pattern) => {
                unhandled_normalization_path!(pattern, "lambda normalization")
            }
            RNixExpr::Root(root) => self.normalize_root(root),
            RNixExpr::AttrSet(attr_set) => self.normalize_attr_set(attr_set),
            RNixExpr::KeyValue(key_value) => {
                unhandled_normalization_path!(key_value, "attr set normalization")
            }
            RNixExpr::Str(str) => self.normalize_str(str),
            RNixExpr::StrInterpol(str_interpol) => {
                unhandled_normalization_path!(str_interpol, "string normalization")
            }
            RNixExpr::UnaryOp(unary_op) => self.normalize_unary_op(unary_op),
            RNixExpr::Value(value) => self.normalize_value(value),
            RNixExpr::With(with) => self.normalize_with(with),
            RNixExpr::PathWithInterpol(path_with_interpol) => {
                self.normalize_path_with_interpol(path_with_interpol)
            }
        }
    }

    fn boxed_normalize(&self, expr: RNixExpr) -> Box<NormalNixExpr> {
        Box::new(self.normalize(expr))
    }

    // Normalize by squashing nested Apply nodes to a single Call node, collecting function arguments into a list
    fn normalize_apply(&self, apply: Apply) -> NormalNixExpr {
        let mut fun = *apply.lambda;
        let mut args: Vec<NormalNixExpr> = vec![self.normalize(*apply.value)];

        while let RNixExpr::Apply(Apply { lambda, value }) = fun {
            args.push(self.normalize(*value));
            fun = *lambda;
        }

        args.reverse();

        NormalNixExpr::Call {
            fun: self.boxed_normalize(fun),
            args,
        }
    }

    fn normalize_assert(&self, assert: Assert) -> NormalNixExpr {
        NormalNixExpr::Assert {
            cond: self.boxed_normalize(*assert.condition),
            body: self.boxed_normalize(*assert.body),
        }
    }

    fn normalize_ident(&self, ident: Ident) -> NormalNixExpr {
        NormalNixExpr::Var(ident.inner)
    }

    fn normalize_if_else(&self, if_else: IfElse) -> NormalNixExpr {
        NormalNixExpr::If {
            cond: self.boxed_normalize(*if_else.condition),
            then: self.boxed_normalize(*if_else.body),
            else_: self.boxed_normalize(*if_else.else_body),
        }
    }

    fn normalize_select(&self, select: Select) -> NormalNixExpr {
        let mut subject = *select.set;
        let mut path: Vec<AttrName> = vec![self.index_to_atrr_name(*select.index)];

        while let RNixExpr::Select(Select { set, index }) = subject {
            path.push(self.index_to_atrr_name(*index));
            subject = *set;
        }

        path.reverse();

        NormalNixExpr::Select {
            subject: self.boxed_normalize(subject),
            or_default: None,
            path,
        }
    }

    fn normalize_lambda(&self, lambda: Lambda) -> NormalNixExpr {
        let (arg, formals) = match *lambda.arg {
            RNixExpr::Ident(ident) => (Some(ident.inner), None),
            RNixExpr::Pattern(pattern) => {
                let at = pattern.at.map(|at| at.inner);
                let formals = Formals {
                    ellipsis: pattern.ellipsis,
                    entries: pattern
                        .entries
                        .into_iter()
                        .map(|entry| Formal {
                            name: entry.name.inner,
                            default: entry.default.map(|default| self.normalize(*default)),
                        })
                        .collect(),
                };

                (at, Some(formals))
            }
            _ => unreachable!(),
        };

        NormalNixExpr::Lambda {
            arg,
            formals,
            body: self.boxed_normalize(*lambda.body),
        }
    }

    fn normalize_legacy_let(&self, legacy_let: LegacyLet) -> NormalNixExpr {
        NormalNixExpr::Select {
            subject: Box::new(self.normalize_attr_set(AttrSet {
                entries: legacy_let.entries,
                recursive: true, // The attr set of a legacy let is implicitly recursive
            })),
            or_default: None,
            path: vec![AttrName::Symbol("body".to_string())],
        }
    }

    fn normalize_let_in(&self, let_in: LetIn) -> NormalNixExpr {
        NormalNixExpr::Let {
            attrs: Box::new(self.normalize_attr_set(AttrSet {
                entries: let_in.entries,
                recursive: false, // TODO: can let be rec?
            })),
            body: self.boxed_normalize(*let_in.body),
        }
    }

    fn normalize_list(&self, list: List) -> NormalNixExpr {
        NormalNixExpr::List(list.items.into_iter().map(|e| self.normalize(e)).collect())
    }

    fn normalize_bin_op(&self, bin_op: BinOp) -> NormalNixExpr {
        let lhs = *bin_op.lhs;
        let rhs = *bin_op.rhs;
        match bin_op.operator {
            BinOpKind::Concat => {
                NormalNixExpr::OpConcatLists(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
            BinOpKind::IsSet => NormalNixExpr::OpHasAttr {
                subject: self.boxed_normalize(lhs),
                path: {
                    match self.normalize(rhs) {
                        NormalNixExpr::Var(var) => vec![AttrName::Symbol(var)],
                        NormalNixExpr::Select { subject, path, .. } => match *subject {
                            NormalNixExpr::Var(var) => std::iter::once(AttrName::Symbol(var))
                                .chain(path.into_iter())
                                .collect(),
                            _ => unreachable!(), // TODO: I think?
                        },
                        _ => unreachable!(), // TODO: I think?
                    }
                },
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
    fn normalize_or_default(&self, or_default: OrDefault) -> NormalNixExpr {
        // TODO: kinda sucks
        match self.normalize_select(or_default.index) {
            NormalNixExpr::Select {
                subject,
                or_default: None,
                path,
            } => NormalNixExpr::Select {
                subject,
                or_default: Some(self.boxed_normalize(*or_default.default)),
                path,
            },
            _ => unreachable!(),
        }
    }

    fn normalize_paren(&self, paren: Paren) -> NormalNixExpr {
        // The ref impl has no concept of parens, so simply discard it
        self.normalize(*paren.inner)
    }

    fn normalize_root(&self, root: Root) -> NormalNixExpr {
        // The ref impl has no concept of a root, so simply discard it
        self.normalize(*root.inner)
    }

    fn normalize_attr_set(&self, attr_set: AttrSet) -> NormalNixExpr {
        let mut attrs = vec![];
        let mut dynamic_attrs = vec![];

        // For each entry, we generate some number of either dynamic or non-dynamic attrs
        for entry in attr_set.entries {
            match entry {
                // If the entry is of the form `foo = bar`
                Entry::KeyValue(KeyValue { mut key, value }) => {
                    let key_head = key.path.remove(0);
                    let key_tail = key.path;

                    let value = if key_tail.len() > 0 {
                        // If the entry is of the form `x.y.z = bar`, then we expand into `x = { y.z = bar }` and recurse
                        self.normalize_attr_set(AttrSet {
                            entries: vec![Entry::KeyValue(KeyValue {
                                key: Key { path: key_tail },
                                value,
                            })],
                            recursive: false,
                        })
                    } else {
                        // Otherwise, the value of the attr is simply the rhs of the equals as-is
                        self.normalize(*value)
                    };

                    match key_head {
                        // A Dynamic KeyPart is _definitely_ dynamic... _unless_ it's just a string with no interpolations (e.g., `${"foo"}`)! I know...
                        RNixExpr::Dynamic(dynamic) => match self.normalize(*dynamic.inner) {
                            NormalNixExpr::String(name) => {
                                attrs.push(AttrDef {
                                    name,
                                    inherited: false,
                                    expr: value,
                                });
                            }
                            name_expr => {
                                dynamic_attrs.push(DynamicAttrDef {
                                    name_expr,
                                    value_expr: value,
                                });
                            }
                        },
                        // If the key expression is a string, it might be dynamic
                        RNixExpr::Str(str) => match self.normalize_str(str) {
                            // If it turns out to just be a string, it's not dynamic
                            NormalNixExpr::String(name) => {
                                attrs.push(AttrDef {
                                    name,
                                    inherited: false,
                                    expr: value,
                                });
                            }
                            // If it had interpolations in it, it normalized to OpConcatStrings, and is dynamic
                            name_expr @ NormalNixExpr::OpConcatStrings { .. } => dynamic_attrs
                                .push(DynamicAttrDef {
                                    name_expr,
                                    value_expr: value,
                                }),
                            // TODO: normalize_str can't return anything else, but we should represent this impossibility more nicely
                            _ => unreachable!(),
                        },
                        // If the key expression is an identifier, it's not dynamic
                        RNixExpr::Ident(Ident { inner: name }) => {
                            attrs.push(AttrDef {
                                name,
                                inherited: false,
                                expr: value,
                            });
                        }
                        _ => todo!(),
                    }
                }
                // If the entry is of the form `inherit foo`
                Entry::Inherit(Inherit { from, idents }) => {
                    let subject = from.map(|from| self.boxed_normalize(*from.inner));

                    let inherit_attrs: Vec<AttrDef> = match subject {
                        None => idents
                            .into_iter()
                            .map(|ident| AttrDef {
                                name: ident.inner.clone(),
                                inherited: true,
                                expr: NormalNixExpr::Var(ident.inner),
                            })
                            .collect(),
                        Some(subject) => idents
                            .into_iter()
                            .map(|ident| AttrDef {
                                name: ident.inner.clone(),
                                inherited: false,
                                expr: NormalNixExpr::Select {
                                    subject: subject.clone(),
                                    or_default: None,
                                    path: vec![AttrName::Symbol(ident.inner)],
                                },
                            })
                            .collect(),
                    };

                    attrs.extend(inherit_attrs);
                }
            }
        }

        // TODO merge duplicate keys

        // Sort attrs by key names. See attr_set_key_sorting test for explanation.
        attrs.sort_by(|a, b| a.name.cmp(&b.name));

        NormalNixExpr::Attrs {
            rec: attr_set.recursive,
            attrs,
            dynamic_attrs,
        }
    }

    fn normalize_str(&self, str: Str) -> NormalNixExpr {
        // If any of the parts are Ast, then this string has interoplations in it
        if str.parts.iter().any(|part| matches!(part, StrPart::Ast(_))) {
            // The reference impl treats string interpolation as string concatenation with force_string: true
            NormalNixExpr::OpConcatStrings {
                force_string: true,
                es: str
                    .parts
                    .into_iter()
                    .map(|part| match part {
                        StrPart::Literal(lit) => NormalNixExpr::String(lit),
                        StrPart::Ast(expr) => self.normalize(*expr.inner),
                    })
                    .collect(),
            }
        } else {
            // otherwise, there should either be only be one part which is a literal or nothing which indicates an empty string
            match &*str.parts {
                [StrPart::Literal(lit)] => NormalNixExpr::String(lit.to_string()),
                [] => NormalNixExpr::String("".to_string()),
                _ => unreachable!(),
            }
        }
    }

    fn normalize_unary_op(&self, unary_op: UnaryOp) -> NormalNixExpr {
        match unary_op.operator {
            UnaryOpKind::Invert => NormalNixExpr::OpNot(self.boxed_normalize(*unary_op.value)),
            // The reference parser treats negation as subtraction from 0
            UnaryOpKind::Negate => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__sub".to_string())),
                args: vec![NormalNixExpr::Int(0), self.normalize(*unary_op.value)],
            },
        }
    }

    fn normalize_path(&self, path: Path) -> NormalNixExpr {
        match path.anchor {
            Anchor::Absolute => NormalNixExpr::Path(path.path),
            Anchor::Relative => {
                let s = if path.path == "./." {
                    "".to_string()
                } else {
                    format!("/{}", path.path.strip_prefix("./").unwrap_or(&path.path))
                };

                NormalNixExpr::Path(format!("{}{}", self.base_path, s))
            }
            Anchor::Home => NormalNixExpr::Path(format!("{}/{}", self.home_path, path.path)),
            // The reference impl treats store paths as a call to __findFile with the args __nixPath and the path
            Anchor::Store => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__findFile".to_string())),
                args: vec![
                    NormalNixExpr::Var("__nixPath".to_string()),
                    NormalNixExpr::String(path.path),
                ],
            },
        }
    }

    fn normalize_value(&self, value: NixValue) -> NormalNixExpr {
        match value {
            NixValue::Float(nf) => NormalNixExpr::Float(nf),
            NixValue::Integer(n) => NormalNixExpr::Int(n),
            NixValue::String(s) => NormalNixExpr::String(s),
            NixValue::Path(path) => self.normalize_path(path),
        }
    }

    fn normalize_with(&self, with: With) -> NormalNixExpr {
        NormalNixExpr::With {
            attrs: self.boxed_normalize(*with.namespace),
            body: self.boxed_normalize(*with.body),
        }
    }

    fn normalize_path_with_interpol(&self, path_with_interpol: PathWithInterpol) -> NormalNixExpr {
        // The reference impl treats path interpolation as string concatenation of all of the interpolated parts with the first part being expanded into a Path
        let base_path = self.normalize_path(path_with_interpol.base_path);

        let parts = path_with_interpol
            .parts
            .into_iter()
            .skip(1) // skip the first part since we took care of that above
            .map(|part| match part {
                PathPart::Literal(lit) => NormalNixExpr::String(lit),
                PathPart::Ast(expr) => self.normalize(*expr.inner),
            });

        NormalNixExpr::OpConcatStrings {
            force_string: false,
            es: std::iter::once(base_path)
                .chain(parts.into_iter())
                .collect(),
        }
    }

    fn index_to_atrr_name(&self, index: RNixExpr) -> AttrName {
        match self.normalize(index) {
            NormalNixExpr::String(s) | NormalNixExpr::Var(s) => AttrName::Symbol(s),
            expr @ NormalNixExpr::OpConcatStrings { .. } => AttrName::Expr(expr),
            _ => unreachable!(),
        }
    }
}
