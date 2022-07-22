pub use rnix::{
    types::{BinOpKind, UnaryOpKind},
    value::{Anchor, Path},
    NixValue,
};

#[derive(Clone, Debug, PartialEq)]
pub enum RNixExpr {
    Apply(Apply),
    Assert(Assert),
    Key(Key),
    Dynamic(Dynamic),
    // Error, // Error is intentionally omitted, because we only care about parsing well-formed Nix
    Ident(Ident),
    IfElse(IfElse),
    Select(Select),
    Inherit(Inherit),
    InheritFrom(InheritFrom),
    Lambda(Lambda),
    LegacyLet(LegacyLet),
    LetIn(LetIn),
    List(List),
    BinOp(BinOp),
    Paren(Paren),
    PatBind(PatBind),
    PatEntry(PatEntry),
    Pattern(Pattern),
    Root(Root),
    AttrSet(AttrSet),
    KeyValue(KeyValue),
    Str(Str),
    StrInterpol(StrInterpol),
    UnaryOp(UnaryOp),
    Value(NixValue),
    With(With),
    PathWithInterpol(PathWithInterpol),
    HasAttr(HasAttr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Apply {
    pub lambda: Box<RNixExpr>,
    pub value: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Assert {
    pub condition: Box<RNixExpr>,
    pub body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub path: Vec<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dynamic {
    pub inner: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub inner: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfElse {
    pub condition: Box<RNixExpr>,
    pub body: Box<RNixExpr>,
    pub else_body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    pub set: Box<RNixExpr>,
    pub key: Key,
    pub default: Option<Box<RNixExpr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inherit {
    pub from: Option<InheritFrom>,
    pub idents: Vec<Ident>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InheritFrom {
    pub inner: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lambda {
    pub arg: Box<RNixExpr>,
    pub body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyLet {
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LetIn {
    pub entries: Vec<Entry>,
    pub body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub items: Vec<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinOp {
    pub lhs: Box<RNixExpr>,
    pub operator: BinOpKind,
    pub rhs: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Paren {
    pub inner: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatBind {
    pub name: Ident,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatEntry {
    pub name: Ident,
    pub default: Option<Box<RNixExpr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub entries: Vec<PatEntry>,
    pub at: Option<Ident>,
    pub ellipsis: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Root {
    pub inner: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrSet {
    pub entries: Vec<Entry>,
    pub recursive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyValue {
    pub key: Key,
    pub value: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Str {
    pub parts: Vec<StrPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrInterpol {
    pub inner: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOp {
    pub operator: UnaryOpKind,
    pub value: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct With {
    pub namespace: Box<RNixExpr>,
    pub body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathWithInterpol {
    pub base_path: Path,
    pub parts: Vec<PathPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HasAttr {
    pub set: Box<RNixExpr>,
    pub key: Key,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    KeyValue(KeyValue),
    Inherit(Inherit),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Literal(String),
    Ast(StrInterpol),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathPart {
    Literal(String),
    Ast(StrInterpol),
}