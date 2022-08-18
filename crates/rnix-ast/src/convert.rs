use std::{fmt, num};

use rnix::{ast::AstToken, parser::ParseError, Parse, Root};

use crate::ast::{self, RNixExpr};

#[derive(Debug)]
pub enum ToAstError {
    EmptyBranch(String),
    ParseError(ParseError),
    ParseFloatError(num::ParseFloatError),
    ParseIntError(num::ParseIntError),
}

impl fmt::Display for ToAstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToAstError::EmptyBranch(when) => {
                write!(f, "A branch of the rnix AST was empty: {when}")
            }
            ToAstError::ParseError(e) => {
                write!(f, "There was an error in the rnix AST: {e}")
            }
            ToAstError::ParseFloatError(e) => {
                write!(f, "Error parsing float: {e}")
            }
            ToAstError::ParseIntError(e) => {
                write!(f, "Error prarsing int: {e}")
            }
        }
    }
}

impl std::error::Error for ToAstError {}

impl TryFrom<Parse<Root>> for RNixExpr {
    type Error = ToAstError;

    fn try_from(value: Parse<Root>) -> Result<Self, Self::Error> {
        let value =
            value
                .ok()
                .map_err(ToAstError::ParseError)?
                .expr()
                .ok_or(ToAstError::EmptyBranch(
                    "Root has no inner expression".to_string(),
                ))?;
        RNixExpr::try_from(value)
    }
}

macro_rules! try_convert {
    ($e:expr) => {
        try_convert_with!($e, |value| Ok(Box::new(RNixExpr::try_from(value)?)))
    };
}

macro_rules! try_convert_all_with {
    ($e:expr, $f:expr) => {
        $e.map($f).collect::<Result<Vec<_>, _>>()?
    };
}

macro_rules! try_convert_with {
    ($e:expr, $f:expr) => {
        $e.ok_or(ToAstError::EmptyBranch(format!(
            "when converting here: {}:{}",
            line!(),
            column!()
        )))
        .and_then($f)?
    };
}

impl TryFrom<rnix::ast::Expr> for RNixExpr {
    type Error = ToAstError;

    fn try_from(value: rnix::ast::Expr) -> Result<Self, Self::Error> {
        match value {
            rnix::ast::Expr::Apply(apply) => convert_apply(apply).map(RNixExpr::Apply),
            rnix::ast::Expr::Assert(assert) => convert_assert(assert).map(RNixExpr::Assert),
            rnix::ast::Expr::Error(_) => unreachable!(
                "This should have been caught by impl TryFrom<Parse<Root>> for RNixExpr"
            ),
            rnix::ast::Expr::IfElse(if_else) => convert_if_else(if_else).map(RNixExpr::IfElse),
            rnix::ast::Expr::Select(select) => convert_select(select).map(RNixExpr::Select),
            rnix::ast::Expr::Str(str) => convert_str(str).map(RNixExpr::Str),
            rnix::ast::Expr::Path(path) => convert_path(path).map(RNixExpr::Path),
            rnix::ast::Expr::Literal(literal) => convert_literal(literal).map(RNixExpr::Literal),
            rnix::ast::Expr::Lambda(lambda) => convert_lambda(lambda).map(RNixExpr::Lambda),
            rnix::ast::Expr::LegacyLet(legacy_let) => {
                convert_legacy_let(legacy_let).map(RNixExpr::LegacyLet)
            }
            rnix::ast::Expr::LetIn(let_in) => convert_let_in(let_in).map(RNixExpr::LetIn),
            rnix::ast::Expr::List(list) => convert_list(list).map(RNixExpr::List),
            rnix::ast::Expr::BinOp(bin_op) => convert_bin_op(bin_op).map(RNixExpr::BinOp),
            rnix::ast::Expr::Paren(paren) => convert_paren(paren).map(RNixExpr::Paren),
            rnix::ast::Expr::Root(root) => convert_root(root).map(RNixExpr::Root),
            rnix::ast::Expr::AttrSet(attr_set) => convert_attr_set(attr_set).map(RNixExpr::AttrSet),
            rnix::ast::Expr::UnaryOp(unary_op) => convert_unary_op(unary_op).map(RNixExpr::UnaryOp),
            rnix::ast::Expr::Ident(ident) => convert_ident(ident).map(RNixExpr::Ident),
            rnix::ast::Expr::With(with) => convert_with(with).map(RNixExpr::With),
            rnix::ast::Expr::HasAttr(has_attr) => convert_has_attr(has_attr).map(RNixExpr::HasAttr),
        }
    }
}

fn convert_apply(apply: rnix::ast::Apply) -> Result<ast::Apply, ToAstError> {
    Ok(ast::Apply {
        lambda: try_convert!(apply.lambda()),
        argument: try_convert!(apply.argument()),
    })
}

fn convert_assert(assert: rnix::ast::Assert) -> Result<ast::Assert, ToAstError> {
    Ok(ast::Assert {
        condition: try_convert!(assert.condition()),
        body: try_convert!(assert.body()),
    })
}

fn convert_ident(ident: rnix::ast::Ident) -> Result<ast::Ident, ToAstError> {
    Ok(ast::Ident {
        inner: ident.to_string(),
    })
}

fn convert_if_else(if_else: rnix::ast::IfElse) -> Result<ast::IfElse, ToAstError> {
    Ok(ast::IfElse {
        condition: try_convert!(if_else.condition()),
        body: try_convert!(if_else.body()),
        else_body: try_convert!(if_else.else_body()),
    })
}

fn convert_select(select: rnix::ast::Select) -> Result<ast::Select, ToAstError> {
    Ok(ast::Select {
        expr: try_convert!(select.expr()),
        attrpath: try_convert_with!(select.attrpath(), convert_attrpath),
        default_expr: select
            .default_expr()
            .map(|default| RNixExpr::try_from(default))
            .transpose()?
            .map(|default| Box::new(default)),
    })
}

fn convert_inherit(inherit: rnix::ast::Inherit) -> Result<ast::Inherit, ToAstError> {
    Ok(ast::Inherit {
        from: inherit.from().map(convert_inherit_from).transpose()?,
        idents: try_convert_all_with!(inherit.idents(), convert_ident),
    })
}

fn convert_inherit_from(
    inherit_from: rnix::ast::InheritFrom,
) -> Result<ast::InheritFrom, ToAstError> {
    Ok(ast::InheritFrom {
        expr: try_convert!(inherit_from.expr()),
    })
}

fn convert_literal(literal: rnix::ast::Literal) -> Result<ast::Literal, ToAstError> {
    Ok(ast::Literal {
        kind: match literal.kind() {
            rnix::ast::LiteralKind::Float(float) => {
                ast::LiteralKind::Float(float.value().map_err(ToAstError::ParseFloatError)?)
            }
            rnix::ast::LiteralKind::Integer(integer) => {
                ast::LiteralKind::Integer(integer.value().map_err(ToAstError::ParseIntError)?)
            }
            rnix::ast::LiteralKind::Uri(uri) => ast::LiteralKind::Uri(uri.to_string()),
        },
    })
}

fn convert_lambda(lambda: rnix::ast::Lambda) -> Result<ast::Lambda, ToAstError> {
    Ok(ast::Lambda {
        param: try_convert_with!(lambda.param(), convert_param),
        body: try_convert!(lambda.body()),
    })
}

fn convert_legacy_let(legacy_let: rnix::ast::LegacyLet) -> Result<ast::LegacyLet, ToAstError> {
    Ok(ast::LegacyLet {
        entries: entries_from_holder(&legacy_let)?,
    })
}

fn convert_let_in(let_in: rnix::ast::LetIn) -> Result<ast::LetIn, ToAstError> {
    Ok(ast::LetIn {
        entries: entries_from_holder(&let_in)?,
        body: try_convert!(let_in.body()),
    })
}

fn convert_list(list: rnix::ast::List) -> Result<ast::List, ToAstError> {
    Ok(ast::List {
        items: try_convert_all_with!(list.items(), RNixExpr::try_from),
    })
}

fn convert_bin_op(bin_op: rnix::ast::BinOp) -> Result<ast::BinOp, ToAstError> {
    Ok(ast::BinOp {
        lhs: try_convert!(bin_op.lhs()),
        operator: bin_op
            .operator()
            .ok_or(ToAstError::EmptyBranch("BinOp has no operator".to_string()))?,
        rhs: try_convert!(bin_op.rhs()),
    })
}

fn convert_paren(paren: rnix::ast::Paren) -> Result<ast::Paren, ToAstError> {
    Ok(ast::Paren {
        expr: try_convert!(paren.expr()),
    })
}

fn convert_root(root: rnix::ast::Root) -> Result<ast::Root, ToAstError> {
    Ok(ast::Root {
        expr: try_convert!(root.expr()),
    })
}

fn convert_attr_set(attr_set: rnix::ast::AttrSet) -> Result<ast::AttrSet, ToAstError> {
    Ok(ast::AttrSet {
        entries: entries_from_holder(&attr_set)?,
        recursive: attr_set.rec_token().is_some(),
    })
}

fn convert_str(str: rnix::ast::Str) -> Result<ast::Str, ToAstError> {
    Ok(ast::Str {
        parts: try_convert_all_with!(str.normalized_parts().into_iter(), convert_interpol_part),
    })
}

fn convert_interpol(str_interpol: rnix::ast::Interpol) -> Result<ast::StrInterpol, ToAstError> {
    Ok(ast::StrInterpol {
        expr: try_convert!(str_interpol.expr()),
    })
}

fn convert_unary_op(unary_op: rnix::ast::UnaryOp) -> Result<ast::UnaryOp, ToAstError> {
    Ok(ast::UnaryOp {
        operator: unary_op.operator().ok_or(ToAstError::EmptyBranch(
            "UnaryOp has no operator".to_string(),
        ))?,
        expr: try_convert!(unary_op.expr()),
    })
}

fn convert_with(with: rnix::ast::With) -> Result<ast::With, ToAstError> {
    Ok(ast::With {
        namespace: try_convert!(with.namespace()),
        body: try_convert!(with.body()),
    })
}

fn convert_path(path: rnix::ast::Path) -> Result<ast::Path, ToAstError> {
    Ok(ast::Path {
        parts: try_convert_all_with!(path.parts().into_iter(), |part| {
            Ok(match part {
                rnix::ast::InterpolPart::Literal(literal) => {
                    ast::InterpolPart::Literal(literal.syntax().text().to_string())
                }
                rnix::ast::InterpolPart::Interpolation(interpol) => {
                    ast::InterpolPart::Interpolation(convert_interpol(interpol)?)
                }
            })
        }),
    })
}

fn convert_has_attr(has_attr: rnix::ast::HasAttr) -> Result<ast::HasAttr, ToAstError> {
    Ok(ast::HasAttr {
        expr: try_convert!(has_attr.expr()),
        attrpath: try_convert_with!(has_attr.attrpath(), convert_attrpath),
    })
}

fn convert_interpol_part<T>(
    part: rnix::ast::InterpolPart<T>,
) -> Result<ast::InterpolPart<T>, ToAstError> {
    Ok(match part {
        rnix::ast::InterpolPart::Literal(lit) => ast::InterpolPart::Literal(lit),
        rnix::ast::InterpolPart::Interpolation(interpol) => {
            ast::InterpolPart::Interpolation(convert_interpol(interpol)?)
        }
    })
}

fn entries_from_holder(
    entry_holder: &impl rnix::ast::HasEntry,
) -> Result<Vec<ast::Entry>, ToAstError> {
    Ok(try_convert_all_with!(entry_holder.entries(), convert_entry))
}

fn convert_entry(entry: rnix::ast::Entry) -> Result<ast::Entry, ToAstError> {
    match entry {
        rnix::ast::Entry::Inherit(inherit) => Ok(ast::Entry::Inherit(convert_inherit(inherit)?)),
        rnix::ast::Entry::AttrpathValue(attrpath_value) => Ok(ast::Entry::AttrpathValue(
            convert_attrpath_value(attrpath_value)?,
        )),
    }
}

fn convert_attrpath_value(
    attrpath_value: rnix::ast::AttrpathValue,
) -> Result<ast::AttrpathValue, ToAstError> {
    Ok(ast::AttrpathValue {
        attrpath: try_convert_with!(attrpath_value.attrpath(), convert_attrpath),
        value: try_convert!(attrpath_value.value()),
    })
}

fn convert_param(param: rnix::ast::Param) -> Result<ast::Param, ToAstError> {
    match param {
        rnix::ast::Param::Pattern(pattern) => convert_pattern(pattern).map(ast::Param::Pattern),
        rnix::ast::Param::IdentParam(ident_param) => {
            convert_ident_param(ident_param).map(ast::Param::IdentParam)
        }
    }
}

fn convert_pattern(pattern: rnix::ast::Pattern) -> Result<ast::Pattern, ToAstError> {
    Ok(ast::Pattern {
        pat_entries: try_convert_all_with!(pattern.pat_entries(), convert_pat_entry),
        ellipsis: pattern.ellipsis_token().is_some(),
        pat_bind: pattern
            .pat_bind()
            .map(|pat_bind| convert_pat_bind(pat_bind))
            .transpose()?,
    })
}

fn convert_pat_bind(pat_bind: rnix::ast::PatBind) -> Result<ast::PatBind, ToAstError> {
    Ok(ast::PatBind {
        ident: try_convert_with!(pat_bind.ident(), convert_ident),
    })
}

fn convert_pat_entry(pat_entry: rnix::ast::PatEntry) -> Result<ast::PatEntry, ToAstError> {
    Ok(ast::PatEntry {
        ident: try_convert_with!(pat_entry.ident(), convert_ident),
        default: pat_entry
            .default()
            .map(|default| RNixExpr::try_from(default))
            .transpose()?
            .map(|default| Box::new(default)),
    })
}

fn convert_ident_param(ident_param: rnix::ast::IdentParam) -> Result<ast::IdentParam, ToAstError> {
    Ok(ast::IdentParam {
        ident: try_convert_with!(ident_param.ident(), convert_ident),
    })
}

fn convert_attrpath(attrpath: rnix::ast::Attrpath) -> Result<ast::Attrpath, ToAstError> {
    Ok(ast::Attrpath {
        attrs: try_convert_all_with!(attrpath.attrs(), convert_attr),
    })
}

fn convert_attr(attr: rnix::ast::Attr) -> Result<ast::Attr, ToAstError> {
    match attr {
        rnix::ast::Attr::Ident(ident) => Ok(ast::Attr::Ident(convert_ident(ident)?)),
        rnix::ast::Attr::Dynamic(dynamic) => Ok(ast::Attr::Dynamic(convert_dynamic(dynamic)?)),
        rnix::ast::Attr::Str(str) => Ok(ast::Attr::Str(convert_str(str)?)),
    }
}

fn convert_dynamic(dynamic: rnix::ast::Dynamic) -> Result<ast::Dynamic, ToAstError> {
    Ok(ast::Dynamic {
        expr: try_convert!(dynamic.expr()),
    })
}
