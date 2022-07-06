use rnix::{
    types::{
        BinOpKind, EntryHolder, ParsedType, ParsedTypeError, TokenWrapper, UnaryOpKind, Wrapper,
    },
    value::ValueError,
    NixValue, SyntaxNode, AST,
};

pub(crate) enum AttrEntry {
    KeyValue {
        key: Vec<NixExpr>,
        value: Box<NixExpr>,
    },
    Inherit {
        from: Option<Box<NixExpr>>,
        idents: Vec<String>,
    },
}

pub(crate) enum StrPart {
    Literal(String),
    Ast(NixExpr),
}

pub(crate) enum NixExpr {
    Apply {
        lambda: Box<NixExpr>,
        value: Box<NixExpr>,
    },
    Assert {
        condition: Box<NixExpr>,
        body: Box<NixExpr>,
    },
    Ident(String),
    IfElse {
        condition: Box<NixExpr>,
        body: Box<NixExpr>,
        else_body: Box<NixExpr>,
    },
    Select {
        set: Box<NixExpr>,
        index: Box<NixExpr>,
    },
    Lambda {
        arg: Box<NixExpr>,
        body: Box<NixExpr>,
    },
    LetIn {
        entries: Vec<AttrEntry>,
        body: Box<NixExpr>,
    },
    List(Vec<NixExpr>),
    BinOp {
        lhs: Box<NixExpr>,
        operator: BinOpKind,
        rhs: Box<NixExpr>,
    },
    OrDefault {
        index: Box<NixExpr>, // TODO
        default: Box<NixExpr>,
    },
    AttrSet {
        entries: Vec<AttrEntry>,
        recursive: bool,
    },
    Str {
        parts: Vec<StrPart>,
    },
    UnaryOp {
        operator: UnaryOpKind,
        value: Box<NixExpr>,
    },
    Value(NixValue),
    With {
        namespace: Box<NixExpr>,
        body: Box<NixExpr>,
    },
}

pub enum ToAstError {
    EmptyBranch,
    ParsedTypeError(ParsedTypeError),
    ParseError,
    ValueError(ValueError),
}

impl TryFrom<AST> for NixExpr {
    type Error = ToAstError;

    fn try_from(value: AST) -> Result<Self, Self::Error> {
        return NixExpr::try_from(value.root().inner());
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

impl TryFrom<ParsedType> for NixExpr {
    type Error = ToAstError;

    fn try_from(value: ParsedType) -> Result<Self, Self::Error> {
        match value {
            ParsedType::Apply(apply) => {
                let lambda = try_convert!(apply.lambda());
                let value = try_convert!(apply.value());
                Ok(NixExpr::Apply { lambda, value })
            }
            ParsedType::Assert(assert) => {
                let condition = try_convert!(assert.condition());
                let body = try_convert!(assert.body());
                Ok(NixExpr::Assert { condition, body })
            }
            ParsedType::Key(_) => todo!(),
            ParsedType::Dynamic(_) => todo!(),
            ParsedType::Error(_) => Err(ToAstError::ParseError),
            ParsedType::Ident(ident) => Ok(NixExpr::Ident(ident.as_str().to_string())),
            ParsedType::IfElse(if_else) => {
                let condition = try_convert!(if_else.condition());
                let body = try_convert!(if_else.body());
                let else_body = try_convert!(if_else.else_body());
                Ok(NixExpr::IfElse {
                    condition,
                    body,
                    else_body,
                })
            }
            ParsedType::Select(select) => {
                let set = try_convert!(select.set());
                let index = try_convert!(select.set());
                Ok(NixExpr::Select { set, index })
            }
            ParsedType::Inherit(_) => todo!(),
            ParsedType::InheritFrom(_) => todo!(),
            ParsedType::Lambda(lambda) => {
                let arg = try_convert!(lambda.arg());
                let body = try_convert!(lambda.body());
                Ok(NixExpr::Lambda { arg, body })
            }
            ParsedType::LegacyLet(_) => todo!(),
            ParsedType::LetIn(let_in) => {
                let entries = entries_from_holder(&let_in)?;
                let body = try_convert!(let_in.body());
                Ok(NixExpr::LetIn { entries, body })
            }
            ParsedType::List(list) => {
                let items = list
                    .items()
                    .map(NixExpr::try_from)
                    .collect::<Result<Vec<NixExpr>, ToAstError>>()?;
                Ok(NixExpr::List(items))
            }
            ParsedType::BinOp(bin_op) => {
                let lhs = try_convert!(bin_op.lhs());
                let operator = bin_op.operator().ok_or(ToAstError::EmptyBranch)?;
                let rhs = try_convert!(bin_op.rhs());
                Ok(NixExpr::BinOp { lhs, operator, rhs })
            }
            ParsedType::OrDefault(or_default) => {
                let index = Box::new(
                    or_default
                        .index()
                        .ok_or(ToAstError::EmptyBranch)
                        .map(ParsedType::Select)
                        .and_then(NixExpr::try_from)?,
                );
                let default = try_convert!(or_default.default());
                Ok(NixExpr::OrDefault { index, default })
            }
            ParsedType::Paren(paren) => NixExpr::try_from(paren.inner()),
            ParsedType::PatBind(_) => todo!(),
            ParsedType::PatEntry(_) => todo!(),
            ParsedType::Pattern(_) => todo!(),
            ParsedType::Root(_) => todo!(),
            ParsedType::AttrSet(attr_set) => {
                let entries = entries_from_holder(&attr_set)?;
                let recursive = attr_set.recursive();
                Ok(NixExpr::AttrSet { entries, recursive })
            }
            ParsedType::KeyValue(_) => todo!(),
            ParsedType::Str(str) => {
                let parts = str
                    .parts()
                    .into_iter()
                    .map(|part| {
                        Ok(match part {
                            rnix::StrPart::Literal(literal) => StrPart::Literal(literal),
                            rnix::StrPart::Ast(ast) => {
                                StrPart::Ast(NixExpr::try_from(ast.inner())?)
                            }
                        })
                    })
                    .collect::<Result<Vec<StrPart>, ToAstError>>()?;

                Ok(NixExpr::Str { parts })
            }
            ParsedType::StrInterpol(_) => todo!(),
            ParsedType::UnaryOp(unary_op) => {
                let operator = unary_op.operator().ok_or(ToAstError::EmptyBranch)?;
                let value = try_convert!(unary_op.value());
                Ok(NixExpr::UnaryOp { operator, value })
            }
            ParsedType::Value(value) => {
                let value = value.to_value().map_err(ToAstError::ValueError)?;
                Ok(NixExpr::Value(value))
            }
            ParsedType::With(with) => {
                let namespace = try_convert!(with.namespace());
                let body = try_convert!(with.body());
                Ok(NixExpr::With { namespace, body })
            }
            ParsedType::PathWithInterpol(_) => todo!(),
        }
    }
}

fn entries_from_holder(entry_holder: &impl EntryHolder) -> Result<Vec<AttrEntry>, ToAstError> {
    entry_holder
        .node()
        .children()
        .map(|child| match ParsedType::try_from(child).unwrap() {
            ParsedType::KeyValue(entry) => {
                let key = entry.key().ok_or(ToAstError::EmptyBranch).and_then(|key| {
                    key.path()
                        .map(|part| NixExpr::try_from(part))
                        .collect::<Result<Vec<NixExpr>, ToAstError>>()
                })?;

                let value = try_convert!(entry.value());

                Ok(AttrEntry::KeyValue { key, value })
            }
            ParsedType::Inherit(inherit) => {
                let from = inherit
                    .from()
                    .map(|from| NixExpr::try_from(from.inner()).map(Box::new))
                    .transpose()?;

                let idents = inherit
                    .idents()
                    .map(|ident| ident.as_str().to_string())
                    .collect();

                Ok(AttrEntry::Inherit { from, idents })
            }
            _ => unreachable!(),
        })
        .collect::<Result<Vec<_>, _>>()
}
