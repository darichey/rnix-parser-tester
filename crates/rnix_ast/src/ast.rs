pub use rnix::{
    types::{BinOpKind, UnaryOpKind},
    value::{Anchor, Path},
    NixValue,
};

#[derive(Clone, Debug, PartialEq)]
pub enum NixExpr {
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
    OrDefault(OrDefault),
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct Apply {
    pub lambda: Box<NixExpr>,
    pub value: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Assert {
    pub condition: Box<NixExpr>,
    pub body: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub path: Vec<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dynamic {
    pub inner: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub inner: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfElse {
    pub condition: Box<NixExpr>,
    pub body: Box<NixExpr>,
    pub else_body: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    pub set: Box<NixExpr>,
    pub index: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inherit {
    pub from: Option<InheritFrom>,
    pub idents: Vec<Ident>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InheritFrom {
    pub inner: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lambda {
    pub arg: Box<NixExpr>,
    pub body: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyLet {
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LetIn {
    pub entries: Vec<Entry>,
    pub body: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub items: Vec<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinOp {
    pub lhs: Box<NixExpr>,
    pub operator: BinOpKind,
    pub rhs: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrDefault {
    pub index: Select,
    pub default: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Paren {
    pub inner: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatBind {
    pub name: Ident,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatEntry {
    pub name: Ident,
    pub default: Option<Box<NixExpr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub entries: Vec<PatEntry>,
    pub at: Option<Ident>,
    pub ellipsis: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Root {
    pub inner: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrSet {
    pub entries: Vec<Entry>,
    pub recursive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyValue {
    pub key: Key,
    pub value: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Str {
    pub parts: Vec<StrPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrInterpol {
    pub inner: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOp {
    pub operator: UnaryOpKind,
    pub value: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct With {
    pub namespace: Box<NixExpr>,
    pub body: Box<NixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathWithInterpol {
    pub base_path: Path,
    pub parts: Vec<PathPart>,
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
