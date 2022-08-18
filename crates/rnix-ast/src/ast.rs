pub use rnix::ast::{BinOpKind, UnaryOpKind};

#[derive(Clone, Debug, PartialEq)]
pub enum RNixExpr {
    Apply(Apply),
    Assert(Assert),
    // Error, // Error is intentionally omitted, because we only care about parsing well-formed Nix
    IfElse(IfElse),
    Select(Select),
    Str(Str),
    Path(Path),
    Literal(Literal),
    Lambda(Lambda),
    LegacyLet(LegacyLet),
    LetIn(LetIn),
    List(List),
    BinOp(BinOp),
    Paren(Paren),
    Root(Root),
    AttrSet(AttrSet),
    UnaryOp(UnaryOp),
    Ident(Ident),
    With(With),
    HasAttr(HasAttr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Apply {
    pub lambda: Box<RNixExpr>,
    pub argument: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Assert {
    pub condition: Box<RNixExpr>,
    pub body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfElse {
    pub condition: Box<RNixExpr>,
    pub body: Box<RNixExpr>,
    pub else_body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    pub expr: Box<RNixExpr>,
    pub attrpath: Attrpath,
    pub default_expr: Option<Box<RNixExpr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Str {
    pub parts: Vec<InterpolPart<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub parts: Vec<InterpolPart<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inherit {
    pub from: Option<InheritFrom>,
    pub idents: Vec<Ident>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InheritFrom {
    pub expr: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Literal {
    pub kind: LiteralKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lambda {
    pub param: Param,
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
    pub expr: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Root {
    pub expr: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrSet {
    pub entries: Vec<Entry>,
    pub recursive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOp {
    pub operator: UnaryOpKind,
    pub expr: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub inner: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct With {
    pub namespace: Box<RNixExpr>,
    pub body: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HasAttr {
    pub expr: Box<RNixExpr>,
    pub attrpath: Attrpath,
}

// == Nodes that don't appear at the top level ==

#[derive(Clone, Debug, PartialEq)]
pub struct Attrpath {
    pub attrs: Vec<Attr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Attr {
    Ident(Ident),
    Dynamic(Dynamic),
    Str(Str),
}

#[derive(Clone, Debug, PartialEq)]
pub enum LiteralKind {
    Float(f64),
    Integer(i64),
    Uri(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dynamic {
    pub expr: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    Pattern(Pattern),
    IdentParam(IdentParam),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub pat_entries: Vec<PatEntry>,
    pub ellipsis: bool,
    pub pat_bind: Option<PatBind>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatEntry {
    pub ident: Ident,
    pub default: Option<Box<RNixExpr>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatBind {
    pub ident: Ident,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdentParam {
    pub ident: Ident,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Inherit(Inherit),
    AttrpathValue(AttrpathValue),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrpathValue {
    pub attrpath: Attrpath,
    pub value: Box<RNixExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InterpolPart<T> {
    Literal(T),
    Interpolation(StrInterpol),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrInterpol {
    pub expr: Box<RNixExpr>,
}
