mod value;

use itertools::{chain, Either, Itertools};
use normal_ast::{AttrDef, AttrName, DynamicAttrDef, Formal, Formals, NormalNixExpr};
use rnix_ast::ast::{
    Apply, Assert, Attr, AttrSet, Attrpath, AttrpathValue, BinOp, BinOpKind, Dynamic, Entry,
    HasAttr, Ident, IfElse, Inherit, InterpolPart, Lambda, LegacyLet, LetIn, List, Literal,
    LiteralKind, Param, Paren, Path, RNixExpr, Root, Select, Str, UnaryOp, UnaryOpKind, With,
};
use value::{parse_path, Anchor};

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

impl Normalizer {
    fn normalize(&self, expr: RNixExpr) -> NormalNixExpr {
        match expr {
            RNixExpr::Apply(apply) => self.normalize_apply(apply),
            RNixExpr::Assert(assert) => self.normalize_assert(assert),
            RNixExpr::Ident(ident) => self.normalize_ident(ident),
            RNixExpr::IfElse(if_else) => self.normalize_if_else(if_else),
            RNixExpr::Select(select) => self.normalize_select(select),
            RNixExpr::Str(str) => self.normalize_str(str),
            RNixExpr::Path(path) => self.normalize_path(path),
            RNixExpr::Lambda(lambda) => self.normalize_lambda(lambda),
            RNixExpr::LegacyLet(legacy_let) => self.normalize_legacy_let(legacy_let),
            RNixExpr::LetIn(let_in) => self.normalize_let_in(let_in),
            RNixExpr::List(list) => self.normalize_list(list),
            RNixExpr::BinOp(bin_op) => self.normalize_bin_op(bin_op),
            RNixExpr::Paren(paren) => self.normalize_paren(paren),
            RNixExpr::Root(root) => self.normalize_root(root),
            RNixExpr::AttrSet(attr_set) => self.normalize_attr_set(attr_set),
            RNixExpr::UnaryOp(unary_op) => self.normalize_unary_op(unary_op),
            RNixExpr::Literal(literal) => self.normalize_literal(literal),
            RNixExpr::With(with) => self.normalize_with(with),
            RNixExpr::HasAttr(has_attr) => self.normalize_has_attr(has_attr),
        }
    }

    fn boxed_normalize(&self, expr: RNixExpr) -> Box<NormalNixExpr> {
        Box::new(self.normalize(expr))
    }

    /// Normalize by squashing nested Apply nodes to a single [`NormalNixExpr::Call`] node,
    /// collecting function arguments into a list.
    fn normalize_apply(&self, apply: Apply) -> NormalNixExpr {
        let mut fun: NormalNixExpr = self.normalize(*apply.lambda);
        let last_arg = self.normalize(*apply.argument);

        let mut args: Vec<NormalNixExpr> = vec![];

        while let NormalNixExpr::Call {
            fun: inner_fun,
            args: inner_args,
        } = fun
        {
            args.extend(inner_args);
            fun = *inner_fun;
        }

        args.push(last_arg);

        NormalNixExpr::Call {
            fun: Box::new(fun),
            args,
        }
    }

    /// Normalize trivially by normalizing child expressions and repacking into [`NormalNixExpr::Assert`].
    fn normalize_assert(&self, assert: Assert) -> NormalNixExpr {
        NormalNixExpr::Assert {
            cond: self.boxed_normalize(*assert.condition),
            body: self.boxed_normalize(*assert.body),
        }
    }

    /// Normalize trivially by repacking the inner string into [`NormalNixExpr::Var`].
    fn normalize_ident(&self, ident: Ident) -> NormalNixExpr {
        NormalNixExpr::Var(ident.inner)
    }

    /// Normalize trivially by normalizing child expressions and repacking into [`NormalNixExpr::If`].
    fn normalize_if_else(&self, if_else: IfElse) -> NormalNixExpr {
        NormalNixExpr::If {
            cond: self.boxed_normalize(*if_else.condition),
            then: self.boxed_normalize(*if_else.body),
            else_: self.boxed_normalize(*if_else.else_body),
        }
    }

    /// Normalize most of it trivially by normalizing child expressions and repacking into [`NormalNixExpr::Select`].
    /// The interesting part here is normalizing the key path which is described in `normalize_as_attr_path`.
    fn normalize_select(&self, select: Select) -> NormalNixExpr {
        NormalNixExpr::Select {
            subject: self.boxed_normalize(*select.expr),
            or_default: select
                .default_expr
                .map(|default| self.boxed_normalize(*default)),
            path: self.normalize_attr_path(select.attrpath),
        }
    }

    /// TODO
    fn normalize_lambda(&self, lambda: Lambda) -> NormalNixExpr {
        let (arg, formals) = match lambda.param {
            Param::IdentParam(ident_param) => (Some(ident_param.ident.inner), None),
            Param::Pattern(pattern) => {
                let at = pattern.pat_bind.map(|pat_bind| pat_bind.ident.inner);
                let formals = Formals {
                    ellipsis: pattern.ellipsis,
                    entries: pattern
                        .pat_entries
                        .into_iter()
                        .map(|entry| {
                            (
                                entry.ident.inner,
                                Formal {
                                    default: entry.default.map(|default| self.normalize(*default)),
                                },
                            )
                        })
                        .collect(),
                };

                (at, Some(formals))
            }
        };

        NormalNixExpr::Lambda {
            arg,
            formals,
            body: self.boxed_normalize(*lambda.body),
        }
    }

    /// TODO
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

    /// TODO
    fn normalize_let_in(&self, let_in: LetIn) -> NormalNixExpr {
        NormalNixExpr::Let {
            attrs: Box::new(self.normalize_attr_set(AttrSet {
                entries: let_in.entries,
                recursive: false,
            })),
            body: self.boxed_normalize(*let_in.body),
        }
    }

    /// TODO
    fn normalize_list(&self, list: List) -> NormalNixExpr {
        NormalNixExpr::List(list.items.into_iter().map(|e| self.normalize(e)).collect())
    }

    /// TODO
    fn normalize_bin_op(&self, bin_op: BinOp) -> NormalNixExpr {
        let lhs = *bin_op.lhs;
        let rhs = *bin_op.rhs;
        match bin_op.operator {
            BinOpKind::Concat => {
                NormalNixExpr::OpConcatLists(self.boxed_normalize(lhs), self.boxed_normalize(rhs))
            }
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

    /// TODO
    fn normalize_paren(&self, paren: Paren) -> NormalNixExpr {
        // The ref impl has no concept of parens, so simply discard it
        self.normalize(*paren.expr)
    }

    /// TODO
    fn normalize_root(&self, root: Root) -> NormalNixExpr {
        // The ref impl has no concept of a root, so simply discard it
        self.normalize(*root.expr)
    }

    /// TODO
    fn normalize_attr_set(&self, attr_set: AttrSet) -> NormalNixExpr {
        // For each entry, we generate some number of either dynamic or non-dynamic attrs
        let (attrs, dynamic_attrs): (Vec<Vec<AttrDef>>, Vec<DynamicAttrDef>) =
            attr_set.entries.into_iter().partition_map(|entry| {
                match entry {
                    // If the entry is of the form `foo = bar`
                    Entry::AttrpathValue(AttrpathValue {
                        mut attrpath,
                        value,
                    }) => {
                        let key_head = attrpath.attrs.remove(0);
                        let key_tail = attrpath.attrs;

                        let value = if !key_tail.is_empty() {
                            // If the entry is of the form `x.y.z = bar`, then we expand into `x = { y.z = bar }` and recurse
                            self.normalize_attr_set(AttrSet {
                                entries: vec![Entry::AttrpathValue(AttrpathValue {
                                    attrpath: Attrpath { attrs: key_tail },
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
                        let subject = from.map(|from| self.boxed_normalize(*from.expr));

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

    /// TODO
    fn normalize_str(&self, str: Str) -> NormalNixExpr {
        // If any of the parts are Ast, then this string has interoplations in it
        if str
            .parts
            .iter()
            .any(|part| matches!(part, InterpolPart::Interpolation(_)))
        {
            // The reference impl treats string interpolation as string concatenation with force_string: true
            NormalNixExpr::OpConcatStrings {
                force_string: true,
                es: str
                    .parts
                    .into_iter()
                    .map(|part| match part {
                        InterpolPart::Literal(lit) => NormalNixExpr::String(lit),
                        InterpolPart::Interpolation(str_interpol) => {
                            self.normalize(*str_interpol.expr)
                        }
                    })
                    .collect(),
            }
        } else {
            // otherwise, there should either be only be one part which is a literal or nothing which indicates an empty string
            match &*str.parts {
                [InterpolPart::Literal(lit)] => NormalNixExpr::String(lit.to_string()),
                [] => NormalNixExpr::String("".to_string()),
                other => unreachable!(
                    "String parts contained only multiple separate literals: {other:?}"
                ),
            }
        }
    }

    /// TODO
    fn normalize_unary_op(&self, unary_op: UnaryOp) -> NormalNixExpr {
        match unary_op.operator {
            UnaryOpKind::Invert => NormalNixExpr::OpNot(self.boxed_normalize(*unary_op.expr)),
            // The reference parser treats negation as subtraction from 0
            UnaryOpKind::Negate => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__sub".to_string())),
                args: vec![NormalNixExpr::Int(0), self.normalize(*unary_op.expr)],
            },
        }
    }

    /// TODO
    fn normalize_literal(&self, literal: Literal) -> NormalNixExpr {
        match literal.kind {
            LiteralKind::Float(nf) => NormalNixExpr::Float(nf),
            LiteralKind::Integer(n) => NormalNixExpr::Int(n),
            LiteralKind::Uri(path) => NormalNixExpr::String(path),
        }
    }

    /// TODO
    fn normalize_with(&self, with: With) -> NormalNixExpr {
        NormalNixExpr::With {
            attrs: self.boxed_normalize(*with.namespace),
            body: self.boxed_normalize(*with.body),
        }
    }

    /// TODO
    fn normalize_path(&self, mut path: Path) -> NormalNixExpr {
        // If any of the parts are Interpolations, then the expression is normalized as a string concatenation with force_string: false
        if path
            .parts
            .iter()
            .any(|part| matches!(part, InterpolPart::Interpolation(_)))
        {
            // Extract the first part, which must be a literal, and expand it
            let parts_head = path.parts.remove(0);
            let parts_tail = path.parts;

            let base_path = match parts_head {
                InterpolPart::Literal(literal) => self.normalize_path_literal(literal),
                InterpolPart::Interpolation(_) => {
                    unreachable!("The first part of a Path should always be a literal")
                }
            };

            let parts = parts_tail.into_iter().map(|part| match part {
                InterpolPart::Literal(lit) => NormalNixExpr::String(lit),
                InterpolPart::Interpolation(str_interpol) => self.normalize(*str_interpol.expr),
            });

            NormalNixExpr::OpConcatStrings {
                force_string: false,
                es: std::iter::once(base_path)
                    .chain(parts.into_iter())
                    .collect(),
            }
        } else {
            // otherwise, there should either be only be one part which is a literal. Expand it
            match &*path.parts {
                [InterpolPart::Literal(lit)] => self.normalize_path_literal(lit.to_string()),
                other => unreachable!(
                    "Path parts contained only multiple separate literals or was empty: {other:?}"
                ),
            }
        }
    }

    /// TODO
    fn normalize_has_attr(&self, has_attr: HasAttr) -> NormalNixExpr {
        NormalNixExpr::OpHasAttr {
            subject: self.boxed_normalize(*has_attr.expr),
            path: self.normalize_attr_path(has_attr.attrpath),
        }
    }

    fn normalize_attr_path(&self, attrpath: Attrpath) -> Vec<AttrName> {
        attrpath
            .attrs
            .into_iter()
            .map(|attr| {
                self.normalize_key_part_as(attr, AttrName::Symbol, AttrName::Expr)
                    .into_inner()
            })
            .collect()
    }

    fn normalize_path_literal(&self, literal: String) -> NormalNixExpr {
        let (anchor, path) = parse_path(literal);
        match anchor {
            Anchor::Absolute => NormalNixExpr::Path(canonicalize(path)),
            Anchor::Relative => {
                NormalNixExpr::Path(canonicalize(format!("{}/{}", self.base_path, path)))
            }
            Anchor::Home => NormalNixExpr::Path(format!("{}/{}", self.home_path, path)),
            // The reference impl treats store paths as a call to __findFile with the args __nixPath and the path
            Anchor::Store => NormalNixExpr::Call {
                fun: Box::new(NormalNixExpr::Var("__findFile".to_string())),
                args: vec![
                    NormalNixExpr::Var("__nixPath".to_string()),
                    NormalNixExpr::String(path),
                ],
            },
        }
    }

    fn normalize_key_part_as<ND, D, FND, FD>(
        &self,
        attr: Attr,
        non_dynamic: FND,
        dynamic: FD,
    ) -> Either<ND, D>
    where
        FND: Fn(String) -> ND,
        FD: Fn(NormalNixExpr) -> D,
    {
        match attr {
            // If the expression is a plain identifier, it's definitely not dynamic
            Attr::Ident(Ident { inner }) => Either::Left(non_dynamic(inner)),
            // If the expression is a string, it's...
            Attr::Str(str) => match self.normalize_str(str) {
                // not dynamic if it's just a plain string
                NormalNixExpr::String(s) => Either::Left(non_dynamic(s)),
                // dynamic if it has string interpolations in it
                concat @ NormalNixExpr::OpConcatStrings { .. } => Either::Right(dynamic(concat)),
                other => unreachable!("It shouldn't be possible for normalize_str to return anything else, but it did: {other:?}"),
            },
            // If the expression is of the form `${x}`, it's...
            Attr::Dynamic(Dynamic { expr }) => match self.normalize(*expr) {
                // _not_ dynamic if x is just a plain string (e.g., `${"foo"}`)
                NormalNixExpr::String(s) => Either::Left(non_dynamic(s)),
                // dynamic otherwise
                inner => Either::Right(dynamic(inner)),
            },
        }
    }
}

fn canonicalize(path: String) -> String {
    // Note that trailing slashes can't occur in user-written nix code, but they can appear in arguments to this function when normalizing interpolated paths.
    // For example, when normalizing the path `/foo/${"bar"}`, normalize_path_with_interpol will call us with `"/foo/"`.
    let has_trailing_slash = path.ends_with('/');

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

    let res = res.join("/");
    if res.is_empty() {
        "/".to_string()
    } else {
        res
    }
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
