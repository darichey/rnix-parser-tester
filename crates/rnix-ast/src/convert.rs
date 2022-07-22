use std::fmt;

use rnix::{
    types::{Dynamic, EntryHolder, ParsedType, ParsedTypeError, TokenWrapper, TypedNode, Wrapper},
    value::ValueError,
    SyntaxNode, TextSize, AST,
};

use crate::ast::{self, NixExpr};

#[derive(Debug)]
pub enum ToAstError {
    EmptyBranch,
    ParsedTypeError(ParsedTypeError),
    ParseError,
    ValueError(ValueError),
}

impl fmt::Display for ToAstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToAstError::EmptyBranch => {
                write!(f, "A branch of the rnix AST was empty")
            }
            ToAstError::ParsedTypeError(err) => {
                write!(f, "Error raising to rnix's typed AST: {}", err)
            }
            ToAstError::ParseError => {
                write!(f, "There was an error in the rnix AST")
            }
            ToAstError::ValueError(err) => {
                write!(f, "Error parsing value: {}", err)
            }
        }
    }
}

impl std::error::Error for ToAstError {}

impl TryFrom<AST> for NixExpr {
    type Error = ToAstError;

    fn try_from(value: AST) -> Result<Self, Self::Error> {
        NixExpr::try_from(value.root().inner())
    }
}

impl TryFrom<Option<SyntaxNode>> for NixExpr {
    type Error = ToAstError;

    fn try_from(value: Option<SyntaxNode>) -> Result<Self, Self::Error> {
        match value {
            None => Err(ToAstError::EmptyBranch),
            Some(value) => NixExpr::try_from(value),
        }
    }
}

impl TryFrom<SyntaxNode> for NixExpr {
    type Error = ToAstError;

    fn try_from(value: SyntaxNode) -> Result<Self, Self::Error> {
        match ParsedType::try_from(value) {
            Err(err) => Err(ToAstError::ParsedTypeError(err)),
            Ok(value) => NixExpr::try_from(value),
        }
    }
}

macro_rules! try_convert {
    ($e:expr) => {
        Box::new(NixExpr::try_from($e)?)
    };
}

macro_rules! try_convert_all {
    ($e:expr, $f:expr) => {
        $e.map($f).collect::<Result<Vec<_>, _>>()?
    };
}

macro_rules! try_convert_and_then {
    ($e:expr, $f:expr) => {
        $e.ok_or(ToAstError::EmptyBranch).and_then($f)?
    };
}

impl TryFrom<ParsedType> for NixExpr {
    type Error = ToAstError;

    fn try_from(value: ParsedType) -> Result<Self, Self::Error> {
        match value {
            ParsedType::Apply(apply) => convert_apply(apply).map(NixExpr::Apply),
            ParsedType::Assert(assert) => convert_assert(assert).map(NixExpr::Assert),
            ParsedType::Key(key) => convert_key(key).map(NixExpr::Key),
            ParsedType::Dynamic(dynamic) => convert_dynamic(dynamic).map(NixExpr::Dynamic),
            ParsedType::Error(_) => Err(ToAstError::ParseError),
            ParsedType::Ident(ident) => convert_ident(ident).map(NixExpr::Ident),
            ParsedType::IfElse(if_else) => convert_if_else(if_else).map(NixExpr::IfElse),
            ParsedType::Select(select) => convert_select(select).map(NixExpr::Select),
            ParsedType::Inherit(inherit) => convert_inherit(inherit).map(NixExpr::Inherit),
            ParsedType::InheritFrom(inherit_from) => {
                convert_inherit_from(inherit_from).map(NixExpr::InheritFrom)
            }
            ParsedType::Lambda(lambda) => convert_lambda(lambda).map(NixExpr::Lambda),
            ParsedType::LegacyLet(legacy_let) => {
                convert_legacy_let(legacy_let).map(NixExpr::LegacyLet)
            }
            ParsedType::LetIn(let_in) => convert_let_in(let_in).map(NixExpr::LetIn),
            ParsedType::List(list) => convert_list(list).map(NixExpr::List),
            ParsedType::BinOp(bin_op) => convert_bin_op(bin_op).map(NixExpr::BinOp),
            ParsedType::Paren(paren) => convert_paren(paren).map(NixExpr::Paren),
            ParsedType::PatBind(pat_bind) => convert_pat_bind(pat_bind).map(NixExpr::PatBind),
            ParsedType::PatEntry(pat_entry) => convert_pat_entry(pat_entry).map(NixExpr::PatEntry),
            ParsedType::Pattern(pattern) => convert_pattern(pattern).map(NixExpr::Pattern),
            ParsedType::Root(root) => convert_root(root).map(NixExpr::Root),
            ParsedType::AttrSet(attr_set) => convert_attr_set(attr_set).map(NixExpr::AttrSet),
            ParsedType::KeyValue(key_value) => convert_key_value(key_value).map(NixExpr::KeyValue),
            ParsedType::Str(str) => convert_str(str).map(NixExpr::Str),
            ParsedType::StrInterpol(str_interpol) => {
                convert_str_interpol(str_interpol).map(NixExpr::StrInterpol)
            }
            ParsedType::UnaryOp(unary_op) => convert_unary_op(unary_op).map(NixExpr::UnaryOp),
            ParsedType::Value(value) => convert_value(value).map(NixExpr::Value),
            ParsedType::With(with) => convert_with(with).map(NixExpr::With),
            ParsedType::PathWithInterpol(path_with_interpol) => {
                convert_path_with_interpol(path_with_interpol).map(NixExpr::PathWithInterpol)
            }
            ParsedType::HasAttr(has_attr) => convert_has_attr(has_attr).map(NixExpr::HasAttr),
        }
    }
}

fn convert_apply(apply: rnix::types::Apply) -> Result<ast::Apply, ToAstError> {
    Ok(ast::Apply {
        lambda: try_convert!(apply.lambda()),
        value: try_convert!(apply.value()),
    })
}

fn convert_assert(assert: rnix::types::Assert) -> Result<ast::Assert, ToAstError> {
    Ok(ast::Assert {
        condition: try_convert!(assert.condition()),
        body: try_convert!(assert.body()),
    })
}

fn convert_key(key: rnix::types::Key) -> Result<ast::Key, ToAstError> {
    Ok(ast::Key {
        path: try_convert_all!(key.path(), NixExpr::try_from),
    })
}

fn convert_dynamic(dynamic: Dynamic) -> Result<ast::Dynamic, ToAstError> {
    Ok(ast::Dynamic {
        inner: try_convert!(dynamic.inner()),
    })
}

fn convert_ident(ident: rnix::types::Ident) -> Result<ast::Ident, ToAstError> {
    Ok(ast::Ident {
        inner: ident.to_inner_string(),
    })
}

fn convert_if_else(if_else: rnix::types::IfElse) -> Result<ast::IfElse, ToAstError> {
    Ok(ast::IfElse {
        condition: try_convert!(if_else.condition()),
        body: try_convert!(if_else.body()),
        else_body: try_convert!(if_else.else_body()),
    })
}

fn convert_select(select: rnix::types::Select) -> Result<ast::Select, ToAstError> {
    Ok(ast::Select {
        set: try_convert!(select.set()),
        key: try_convert_and_then!(select.key(), convert_key),
        default: select
            .default()
            .map(|default| Ok(try_convert!(default)))
            .transpose()?,
    })
}

fn convert_inherit(inherit: rnix::types::Inherit) -> Result<ast::Inherit, ToAstError> {
    Ok(ast::Inherit {
        from: inherit.from().map(convert_inherit_from).transpose()?,
        idents: try_convert_all!(inherit.idents(), convert_ident),
    })
}

fn convert_inherit_from(
    inherit_from: rnix::types::InheritFrom,
) -> Result<ast::InheritFrom, ToAstError> {
    Ok(ast::InheritFrom {
        inner: try_convert!(inherit_from.inner()),
    })
}

fn convert_lambda(lambda: rnix::types::Lambda) -> Result<ast::Lambda, ToAstError> {
    Ok(ast::Lambda {
        arg: try_convert!(lambda.arg()),
        body: try_convert!(lambda.body()),
    })
}

fn convert_legacy_let(legacy_let: rnix::types::LegacyLet) -> Result<ast::LegacyLet, ToAstError> {
    Ok(ast::LegacyLet {
        entries: entries_from_holder(&legacy_let)?,
    })
}

fn convert_let_in(let_in: rnix::types::LetIn) -> Result<ast::LetIn, ToAstError> {
    Ok(ast::LetIn {
        entries: entries_from_holder(&let_in)?,
        body: try_convert!(let_in.body()),
    })
}

fn convert_list(list: rnix::types::List) -> Result<ast::List, ToAstError> {
    Ok(ast::List {
        items: try_convert_all!(list.items(), NixExpr::try_from),
    })
}

fn convert_bin_op(bin_op: rnix::types::BinOp) -> Result<ast::BinOp, ToAstError> {
    Ok(ast::BinOp {
        lhs: try_convert!(bin_op.lhs()),
        operator: bin_op.operator().ok_or(ToAstError::EmptyBranch)?,
        rhs: try_convert!(bin_op.rhs()),
    })
}

fn convert_paren(paren: rnix::types::Paren) -> Result<ast::Paren, ToAstError> {
    Ok(ast::Paren {
        inner: try_convert!(paren.inner()),
    })
}

fn convert_pat_bind(pat_bind: rnix::types::PatBind) -> Result<ast::PatBind, ToAstError> {
    Ok(ast::PatBind {
        name: try_convert_and_then!(pat_bind.name(), convert_ident),
    })
}

fn convert_pat_entry(pat_entry: rnix::types::PatEntry) -> Result<ast::PatEntry, ToAstError> {
    Ok(ast::PatEntry {
        name: try_convert_and_then!(pat_entry.name(), convert_ident),
        default: pat_entry
            .default()
            .map(|default| Ok(try_convert!(default)))
            .transpose()?,
    })
}

fn convert_pattern(pattern: rnix::types::Pattern) -> Result<ast::Pattern, ToAstError> {
    Ok(ast::Pattern {
        entries: try_convert_all!(pattern.entries(), convert_pat_entry),
        at: pattern.at().map(convert_ident).transpose()?,
        ellipsis: pattern.ellipsis(),
    })
}

fn convert_root(root: rnix::types::Root) -> Result<ast::Root, ToAstError> {
    Ok(ast::Root {
        inner: try_convert!(root.inner()),
    })
}

fn convert_attr_set(attr_set: rnix::types::AttrSet) -> Result<ast::AttrSet, ToAstError> {
    Ok(ast::AttrSet {
        entries: entries_from_holder(&attr_set)?,
        recursive: attr_set.recursive(),
    })
}

fn convert_key_value(key_value: rnix::types::KeyValue) -> Result<ast::KeyValue, ToAstError> {
    Ok(ast::KeyValue {
        key: try_convert_and_then!(key_value.key(), convert_key),
        value: try_convert!(key_value.value()),
    })
}

fn convert_str(str: rnix::types::Str) -> Result<ast::Str, ToAstError> {
    Ok(ast::Str {
        parts: try_convert_all!(str.parts().into_iter(), convert_str_part),
    })
}

fn convert_str_interpol(
    str_interpol: rnix::types::StrInterpol,
) -> Result<ast::StrInterpol, ToAstError> {
    Ok(ast::StrInterpol {
        inner: try_convert!(str_interpol.inner()),
    })
}

fn convert_unary_op(unary_op: rnix::types::UnaryOp) -> Result<ast::UnaryOp, ToAstError> {
    Ok(ast::UnaryOp {
        operator: unary_op.operator().ok_or(ToAstError::EmptyBranch)?,
        value: try_convert!(unary_op.value()),
    })
}

fn convert_value(value: rnix::types::Value) -> Result<ast::NixValue, ToAstError> {
    value.to_value().map_err(ToAstError::ValueError)
}

fn convert_with(with: rnix::types::With) -> Result<ast::With, ToAstError> {
    Ok(ast::With {
        namespace: try_convert!(with.namespace()),
        body: try_convert!(with.body()),
    })
}

fn convert_path_with_interpol(
    path_with_interpol: rnix::types::PathWithInterpol,
) -> Result<ast::PathWithInterpol, ToAstError> {
    Ok(ast::PathWithInterpol {
        base_path: path_with_interpol
            .base_path()
            .ok_or(ToAstError::EmptyBranch)?
            .map_err(ToAstError::ValueError)?,
        parts: try_convert_all!(path_with_interpol.parts().into_iter(), convert_path_part),
    })
}

fn convert_has_attr(has_attr: rnix::types::HasAttr) -> Result<ast::HasAttr, ToAstError> {
    Ok(ast::HasAttr {
        set: try_convert!(has_attr.set()),
        key: try_convert_and_then!(has_attr.key(), convert_key),
    })
}

fn convert_str_part(part: rnix::StrPart) -> Result<ast::StrPart, ToAstError> {
    Ok(match part {
        rnix::StrPart::Literal(lit) => ast::StrPart::Literal(lit),
        rnix::StrPart::Ast(str_interpol) => ast::StrPart::Ast(convert_str_interpol(str_interpol)?),
    })
}

fn convert_path_part(part: rnix::PathPart) -> Result<ast::PathPart, ToAstError> {
    Ok(match part {
        rnix::PathPart::Literal(lit) => ast::PathPart::Literal(lit),
        rnix::PathPart::Ast(str_interpol) => {
            ast::PathPart::Ast(convert_str_interpol(str_interpol)?)
        }
    })
}

fn entries_from_holder(entry_holder: &impl EntryHolder) -> Result<Vec<ast::Entry>, ToAstError> {
    let key_values = entry_holder.entries().map(|key_value| {
        Ok((
            key_value.node().text_range().start(),
            ast::Entry::KeyValue(convert_key_value(key_value)?),
        ))
    });

    let inherits = entry_holder.inherits().map(|inherit| {
        Ok((
            inherit.node().text_range().start(),
            ast::Entry::Inherit(convert_inherit(inherit)?),
        ))
    });

    let mut ret: Vec<(TextSize, ast::Entry)> =
        key_values.chain(inherits).collect::<Result<Vec<_>, _>>()?;

    ret.sort_by_key(|(size, _)| *size);

    Ok(ret.into_iter().map(|(_, entry)| entry).collect())
}
