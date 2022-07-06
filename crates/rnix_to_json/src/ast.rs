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

pub(crate) struct PatEntry {
    pub name: String,
    pub default: Option<NixExpr>,
}

pub(crate) enum LambdaArg {
    Ident(String),
    Pattern {
        entries: Vec<PatEntry>,
        at: Option<String>,
        ellipsis: bool,
    },
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
        arg: LambdaArg,
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
            ParsedType::Apply(apply) => Ok(NixExpr::Apply {
                lambda: try_convert!(apply.lambda()),
                value: try_convert!(apply.value()),
            }),
            ParsedType::Assert(assert) => Ok(NixExpr::Assert {
                condition: try_convert!(assert.condition()),
                body: try_convert!(assert.body()),
            }),
            ParsedType::Key(_) => todo!(),
            ParsedType::Dynamic(_) => todo!(),
            ParsedType::Error(_) => Err(ToAstError::ParseError),
            ParsedType::Ident(ident) => Ok(NixExpr::Ident(ident.as_str().to_string())),
            ParsedType::IfElse(if_else) => Ok(NixExpr::IfElse {
                condition: try_convert!(if_else.condition()),
                body: try_convert!(if_else.body()),
                else_body: try_convert!(if_else.else_body()),
            }),
            ParsedType::Select(select) => Ok(NixExpr::Select {
                set: try_convert!(select.set()),
                index: try_convert!(select.set()),
            }),
            ParsedType::Inherit(_) => todo!(),
            ParsedType::InheritFrom(_) => todo!(),
            ParsedType::Lambda(lambda) => {
                let arg = lambda
                    .arg()
                    .ok_or(ToAstError::EmptyBranch)
                    .and_then(|arg| ParsedType::try_from(arg).map_err(ToAstError::ParsedTypeError))
                    .and_then(|arg| {
                        Ok(match arg {
                            ParsedType::Ident(ident) => {
                                LambdaArg::Ident(ident.as_str().to_string())
                            }
                            ParsedType::Pattern(pattern) => LambdaArg::Pattern {
                                entries: pattern
                                    .entries()
                                    .map(|entry| {
                                        Ok(PatEntry {
                                            name: entry
                                                .name()
                                                .map(|ident| ident.as_str().to_string())
                                                .ok_or(ToAstError::EmptyBranch)?,
                                            default: entry
                                                .default()
                                                .map(|default| NixExpr::try_from(default))
                                                .transpose()?,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, _>>()?,
                                at: pattern.at().map(|ident| ident.as_str().to_string()),
                                ellipsis: pattern.ellipsis(),
                            },
                            _ => unreachable!(),
                        })
                    })?;

                Ok(NixExpr::Lambda {
                    arg,
                    body: try_convert!(lambda.body()),
                })
            }
            ParsedType::LegacyLet(_) => todo!(),
            ParsedType::LetIn(let_in) => Ok(NixExpr::LetIn {
                entries: entries_from_holder(&let_in)?,
                body: try_convert!(let_in.body()),
            }),
            ParsedType::List(list) => Ok(NixExpr::List(
                list.items()
                    .map(NixExpr::try_from)
                    .collect::<Result<Vec<NixExpr>, ToAstError>>()?,
            )),
            ParsedType::BinOp(bin_op) => Ok(NixExpr::BinOp {
                lhs: try_convert!(bin_op.lhs()),
                operator: bin_op.operator().ok_or(ToAstError::EmptyBranch)?,
                rhs: try_convert!(bin_op.rhs()),
            }),
            ParsedType::OrDefault(or_default) => Ok(NixExpr::OrDefault {
                index: Box::new(
                    or_default
                        .index()
                        .ok_or(ToAstError::EmptyBranch)
                        .map(ParsedType::Select)
                        .and_then(NixExpr::try_from)?,
                ),
                default: try_convert!(or_default.default()),
            }),
            ParsedType::Paren(paren) => NixExpr::try_from(paren.inner()),
            ParsedType::PatBind(_) => todo!(),
            ParsedType::PatEntry(_) => todo!(),
            ParsedType::Pattern(_) => todo!(),
            ParsedType::Root(_) => todo!(),
            ParsedType::AttrSet(attr_set) => Ok(NixExpr::AttrSet {
                entries: entries_from_holder(&attr_set)?,
                recursive: attr_set.recursive(),
            }),
            ParsedType::KeyValue(_) => todo!(),
            ParsedType::Str(str) => Ok(NixExpr::Str {
                parts: str
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
                    .collect::<Result<Vec<StrPart>, ToAstError>>()?,
            }),
            ParsedType::StrInterpol(_) => todo!(),
            ParsedType::UnaryOp(unary_op) => Ok(NixExpr::UnaryOp {
                operator: unary_op.operator().ok_or(ToAstError::EmptyBranch)?,
                value: try_convert!(unary_op.value()),
            }),
            ParsedType::Value(value) => Ok(NixExpr::Value(
                value.to_value().map_err(ToAstError::ValueError)?,
            )),
            ParsedType::With(with) => Ok(NixExpr::With {
                namespace: try_convert!(with.namespace()),
                body: try_convert!(with.body()),
            }),
            ParsedType::PathWithInterpol(_) => todo!(),
        }
    }
}

fn entries_from_holder(entry_holder: &impl EntryHolder) -> Result<Vec<AttrEntry>, ToAstError> {
    entry_holder
        .node()
        .children()
        .map(|child| match ParsedType::try_from(child).unwrap() {
            ParsedType::KeyValue(entry) => Ok(AttrEntry::KeyValue {
                key: entry.key().ok_or(ToAstError::EmptyBranch).and_then(|key| {
                    key.path()
                        .map(|part| NixExpr::try_from(part))
                        .collect::<Result<Vec<NixExpr>, ToAstError>>()
                })?,
                value: try_convert!(entry.value()),
            }),
            ParsedType::Inherit(inherit) => Ok(AttrEntry::Inherit {
                from: inherit
                    .from()
                    .map(|from| NixExpr::try_from(from.inner()).map(Box::new))
                    .transpose()?,
                idents: inherit
                    .idents()
                    .map(|ident| ident.as_str().to_string())
                    .collect(),
            }),
            _ => unreachable!(),
        })
        .collect::<Result<Vec<_>, _>>()
}