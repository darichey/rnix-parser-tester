use serde::Serialize;

#[derive(Clone, Serialize, Debug)]
pub enum NixExpr {
    Int(i64),
    Float(f64),
    String(String),
    Path(String),
    Var(String),
    Select {
        subject: Box<NixExpr>,
        or_default: Option<Box<NixExpr>>,
        path: Vec<AttrName>,
    },
    OpHasAttr {
        subject: Box<NixExpr>,
        path: Vec<AttrName>,
    },
    Attrs {
        rec: bool,
        attrs: Vec<AttrDef>,
        dynamic_attrs: Vec<DynamicAttrDef>,
    },
    List(Vec<NixExpr>),
    Lambda {
        arg: Option<String>,
        formals: Option<Formals>,
        body: Box<NixExpr>,
    },
    Call {
        fun: Box<NixExpr>,
        args: Vec<NixExpr>,
    },
    Let {
        attrs: Box<NixExpr>, // TODO
        body: Box<NixExpr>,
    },
    With {
        attrs: Box<NixExpr>,
        body: Box<NixExpr>,
    },
    If {
        cond: Box<NixExpr>,
        then: Box<NixExpr>,
        else_: Box<NixExpr>,
    },
    Assert {
        cond: Box<NixExpr>,
        body: Box<NixExpr>,
    },
    OpNot(Box<NixExpr>),
    OpEq(Box<NixExpr>, Box<NixExpr>),
    OpNEq(Box<NixExpr>, Box<NixExpr>),
    OpAnd(Box<NixExpr>, Box<NixExpr>),
    OpOr(Box<NixExpr>, Box<NixExpr>),
    OpImpl(Box<NixExpr>, Box<NixExpr>),
    OpUpdate(Box<NixExpr>, Box<NixExpr>),
    OpConcatLists(Box<NixExpr>, Box<NixExpr>),
    OpConcatStrings {
        force_string: bool,
        es: Vec<NixExpr>,
    },
}

#[derive(Clone, Serialize, Debug)]
pub enum AttrName {
    Symbol(String),
    Expr(NixExpr),
}

#[derive(Clone, Serialize, Debug)]
pub struct AttrDef {
    pub name: String,
    pub inherited: bool,
    pub expr: NixExpr,
}

#[derive(Clone, Serialize, Debug)]
pub struct DynamicAttrDef {
    pub name_expr: NixExpr,
    pub value_expr: NixExpr,
}

#[derive(Clone, Serialize, Debug)]
pub struct Formal {
    pub name: String,
    pub default: Option<NixExpr>,
}

#[derive(Clone, Serialize, Debug)]
pub struct Formals {
    pub ellipsis: bool,
    pub entries: Vec<Formal>,
}
