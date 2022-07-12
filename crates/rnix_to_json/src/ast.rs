use nonempty::NonEmpty;
use rnix::{
    types::{
        BinOpKind, Dynamic, EntryHolder, ParsedType, ParsedTypeError, TokenWrapper, UnaryOpKind,
        Wrapper,
    },
    value::{Path, ValueError},
    NixValue, SyntaxKind, SyntaxNode, AST,
};

#[derive(Debug)]
pub(crate) enum KeyPart {
    Dynamic(Box<NixExpr>),
    Plain(Box<NixExpr>),
}

#[derive(Debug)]
pub(crate) enum AttrEntry {
    KeyValue {
        key: NonEmpty<KeyPart>,
        value: Box<NixExpr>,
    },
    Inherit {
        from: Option<Box<NixExpr>>,
        idents: Vec<String>,
    },
}

#[derive(Debug)]
pub(crate) enum StrPart {
    Literal(String),
    Ast(NixExpr),
}

#[derive(Debug)]
pub(crate) struct PatEntry {
    pub name: String,
    pub default: Option<NixExpr>,
}

#[derive(Debug)]
pub(crate) enum LambdaArg {
    Ident(String),
    Pattern {
        entries: Vec<PatEntry>,
        at: Option<String>,
        ellipsis: bool,
    },
}

#[derive(Debug)]
pub(crate) enum PathPart {
    Literal(String),
    Ast(NixExpr),
}

#[derive(Debug)]
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
    PathInterpol {
        base_path: Path,
        parts: Vec<PathPart>,
    },
}

#[derive(Debug)]
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
            ParsedType::Ident(ident) => Ok(NixExpr::Ident(ident.to_inner_string())),
            ParsedType::IfElse(if_else) => Ok(NixExpr::IfElse {
                condition: try_convert!(if_else.condition()),
                body: try_convert!(if_else.body()),
                else_body: try_convert!(if_else.else_body()),
            }),
            ParsedType::Select(select) => Ok(NixExpr::Select {
                set: try_convert!(select.set()),
                index: try_convert!(select.index()),
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
                            ParsedType::Ident(ident) => LambdaArg::Ident(ident.to_inner_string()),
                            ParsedType::Pattern(pattern) => LambdaArg::Pattern {
                                entries: pattern
                                    .entries()
                                    .map(|entry| {
                                        Ok(PatEntry {
                                            name: entry
                                                .name()
                                                .map(|ident| ident.to_inner_string())
                                                .ok_or(ToAstError::EmptyBranch)?,
                                            default: entry
                                                .default()
                                                .map(|default| NixExpr::try_from(default))
                                                .transpose()?,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, _>>()?,
                                at: pattern.at().map(|ident| ident.to_inner_string()),
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
            ParsedType::PathWithInterpol(path) => Ok(NixExpr::PathInterpol {
                base_path: path
                    .base_path()
                    .ok_or(ToAstError::EmptyBranch)?
                    .map_err(ToAstError::ValueError)?,
                parts: path
                    .parts()
                    .into_iter()
                    .map(|part| {
                        Ok(match part {
                            rnix::PathPart::Literal(literal) => PathPart::Literal(literal),
                            rnix::PathPart::Ast(ast) => {
                                PathPart::Ast(NixExpr::try_from(ast.inner())?)
                            }
                        })
                    })
                    .collect::<Result<Vec<PathPart>, ToAstError>>()?,
            }),
        }
    }
}

fn entries_from_holder(entry_holder: &impl EntryHolder) -> Result<Vec<AttrEntry>, ToAstError> {
    entry_holder
        .node()
        .children()
        // Ignore other children. e.g., a let node would have its body as a child too
        .filter(|child| {
            child.kind() == SyntaxKind::NODE_KEY_VALUE || child.kind() == SyntaxKind::NODE_INHERIT
        })
        .map(|child| ParsedType::try_from(child).map_err(ToAstError::ParsedTypeError))
        .map(|child| match child? {
            ParsedType::KeyValue(entry) => Ok(AttrEntry::KeyValue {
                key: NonEmpty::from_vec(entry.key().ok_or(ToAstError::EmptyBranch).and_then(
                    |key| {
                        key.path()
                            .map(|part| {
                                // Mark dynamic entries
                                let part = ParsedType::try_from(part)
                                    .map_err(ToAstError::ParsedTypeError)?;
                                if let ParsedType::Dynamic(dynamic) = part {
                                    Ok(KeyPart::Dynamic(try_convert!(dynamic.inner())))
                                } else {
                                    Ok(KeyPart::Plain(try_convert!(part)))
                                }
                            })
                            .collect::<Result<Vec<KeyPart>, ToAstError>>()
                    },
                )?)
                .ok_or(ToAstError::EmptyBranch)?,
                value: try_convert!(entry.value()),
            }),
            ParsedType::Inherit(inherit) => Ok(AttrEntry::Inherit {
                from: inherit
                    .from()
                    .map(|from| NixExpr::try_from(from.inner()).map(Box::new))
                    .transpose()?,
                idents: inherit
                    .idents()
                    .map(|ident| ident.to_inner_string())
                    .collect(),
            }),
            _ => unreachable!(), // Unreachable because of above filter
        })
        .collect::<Result<Vec<_>, _>>()
}
