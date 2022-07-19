use ast::{AttrDef, AttrName, DynamicAttrDef, Formal, Formals, NixExpr as NormalNixExpr};
use itertools::{chain, Either, Itertools};
use rnix_ast::ast::{
    Anchor, Apply, Assert, AttrSet, BinOp, BinOpKind, Dynamic, Entry, Ident, IfElse, Inherit, Key,
    KeyValue, Lambda, LegacyLet, LetIn, List, NixExpr as RNixExpr, NixValue, OrDefault, Paren,
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
        let (subject, path) = self.linearize_nested_select(select);

        NormalNixExpr::Select {
            subject: self.boxed_normalize(subject),
            or_default: None,
            path: self.normalize_as_attr_path(path),
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
                        .map(|entry| {
                            (
                                entry.name.inner,
                                Formal {
                                    default: entry.default.map(|default| self.normalize(*default)),
                                },
                            )
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
                path: match rhs {
                    RNixExpr::Select(select) => {
                        let (subject, path) = self.linearize_nested_select(select);
                        self.normalize_as_attr_path(std::iter::once(subject).chain(path).collect())
                    }
                    rhs => self.normalize_as_attr_path(vec![rhs]),
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
        // For each entry, we generate some number of either dynamic or non-dynamic attrs
        let (attrs, dynamic_attrs): (Vec<Vec<AttrDef>>, Vec<DynamicAttrDef>) =
            attr_set.entries.into_iter().partition_map(|entry| {
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

                        self.normalize_key_part_as(
                            key_head,
                            |name| {
                                vec![AttrDef {
                                    name,
                                    inherited: false,
                                    expr: value.clone(),
                                }]
                            },
                            |name_expr| DynamicAttrDef {
                                name_expr,
                                value_expr: value.clone(),
                            },
                        )
                    }
                    // If the entry is of the form `inherit foo`
                    Entry::Inherit(Inherit { from, idents }) => {
                        let subject = from.map(|from| self.boxed_normalize(*from.inner));

                        let attrs: Vec<AttrDef> = idents
                            .into_iter()
                            .map(|ident| match &subject {
                                Some(subject) => AttrDef {
                                    name: ident.inner.clone(),
                                    inherited: false,
                                    expr: NormalNixExpr::Select {
                                        subject: subject.clone(),
                                        or_default: None,
                                        path: vec![AttrName::Symbol(ident.inner)],
                                    },
                                },
                                None => AttrDef {
                                    name: ident.inner.clone(),
                                    inherited: true,
                                    expr: NormalNixExpr::Var(ident.inner),
                                },
                            })
                            .collect();

                        Either::Left(attrs)
                    }
                }
            });

        // Sort attrs by key names. See attr_set_key_sorting test for explanation.
        let attrs: Vec<AttrDef> = attrs.into_iter().flatten().collect();

        let attrs = merge_attrs(attrs, vec![]);
        let dynamic_attrs = merge_dynamic_attrs(dynamic_attrs, vec![]);

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

    fn normalize_path(&self, path: rnix_ast::ast::Path) -> NormalNixExpr {
        match path.anchor {
            Anchor::Absolute => NormalNixExpr::Path(canonicalize(path.path)),
            Anchor::Relative => {
                NormalNixExpr::Path(canonicalize(format!("{}/{}", self.base_path, path.path)))
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

    fn linearize_nested_select(&self, select: Select) -> (RNixExpr, Vec<RNixExpr>) {
        let mut subject = *select.set;
        let mut path: Vec<RNixExpr> = vec![*select.index];

        while let RNixExpr::Select(Select { set, index }) = subject {
            path.push(*index);
            subject = *set;
        }

        path.reverse();
        (subject, path)
    }

    fn normalize_as_attr_path(&self, path: Vec<RNixExpr>) -> Vec<AttrName> {
        path.into_iter()
            .map(|expr| {
                self.normalize_key_part_as(expr, AttrName::Symbol, AttrName::Expr)
                    .into_inner()
            })
            .collect()
    }

    fn normalize_key_part_as<ND, D, FND, FD>(
        &self,
        expr: RNixExpr,
        non_dynamic: FND,
        dynamic: FD,
    ) -> Either<ND, D>
    where
        FND: Fn(String) -> ND,
        FD: Fn(NormalNixExpr) -> D,
    {
        match expr {
            // If the expression is a plain identifier, it's definitely not dynamic
            RNixExpr::Ident(Ident { inner }) => Either::Left(non_dynamic(inner)),
            // If the expression is a string, it's...
            RNixExpr::Str(str) => match self.normalize_str(str) {
                // not dynamic if it's just a plain string
                NormalNixExpr::String(s) => Either::Left(non_dynamic(s)),
                // dynamic if it has string interpolations in it
                concat @ NormalNixExpr::OpConcatStrings { .. } => Either::Right(dynamic(concat)),
                other => unreachable!("It shouldn't be possible for normalize_str to return anything else, but it did: {other:?}"),
            },
            // If the expression is of the form `${x}`, it's...
            RNixExpr::Dynamic(Dynamic { inner }) => match self.normalize(*inner) {
                // _not_ dynamic if x is just a plain string (e.g., `${"foo"}`)
                NormalNixExpr::String(s) => Either::Left(non_dynamic(s)),
                // dynamic otherwise
                inner => Either::Right(dynamic(inner)),
            },
            other => unreachable!("It shouldn't be possible for a key path to contain other kinds of expressions, but it did: {other:?}"),
        }
    }
}

fn canonicalize(path: String) -> String {
    // Note that trailing slashes can't occur in user-written nix code, but they can appear in arguments to this function when normalizing interpolated paths.
    // For example, when normalizing the path `/foo/${"bar"}`, normalize_path_with_interpol will call us with `"/foo/"`.
    let has_trailing_slash = path.ends_with("/");

    let mut res = vec![];

    for comp in std::path::Path::new(&path).components() {
        match comp {
            std::path::Component::RootDir => {
                res.push("");
            }
            std::path::Component::ParentDir => {
                res.pop();
            }
            std::path::Component::Normal(s) => {
                res.push(s.to_str().unwrap());
            }
            _ => {}
        }
    }

    if has_trailing_slash {
        res.push("");
    }

    res.join("/")
}

fn merge_attrs(attrs1: Vec<AttrDef>, attrs2: Vec<AttrDef>) -> Vec<AttrDef> {
    attrs1
        .into_iter()
        .chain(attrs2)
        .into_grouping_map_by(|def| def.name.clone())
        .fold_first(merge_attr_def)
        .into_values()
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .collect()
}

fn merge_dynamic_attrs(
    dynamic_attrs1: Vec<DynamicAttrDef>,
    dynamic_attrs2: Vec<DynamicAttrDef>,
) -> Vec<DynamicAttrDef> {
    // Nix disallows overlapping dynamic attrs. For example, `let x = "x"; in { ${x} = {}; ${x} = {}; }` is not legal.
    // However, it doesn't check this until evaluation (because the key must be evaluated). During parsing, it just
    // lumps all dynamic attributes together, like we do here.
    chain!(dynamic_attrs1, dynamic_attrs2).collect()
}

fn merge_attr_def(def1: AttrDef, name: &String, def2: AttrDef) -> AttrDef {
    if def1.inherited || def2.inherited {
        panic!("{name} is inherited, but inherited defs cannot be merged.");
    }

    match (def1.expr, def2.expr) {
        (
            NormalNixExpr::Attrs {
                rec: rec1,
                attrs: attrs1,
                dynamic_attrs: dynamic_attrs1,
            },
            NormalNixExpr::Attrs {
                rec: rec2,
                attrs: attrs2,
                dynamic_attrs: dynamic_attrs2,
            },
        ) => AttrDef {
            name: def1.name, // def1.name == def2.name == name
            inherited: false,
            expr: NormalNixExpr::Attrs {
                rec: rec1 || rec2,
                attrs: merge_attrs(attrs1, attrs2),
                dynamic_attrs: merge_dynamic_attrs(dynamic_attrs1, dynamic_attrs2),
            },
        },
        _ => panic!("Cannot merge {name}, because one of the values is not an attrset"),
    }
}
