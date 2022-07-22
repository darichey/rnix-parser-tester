use std::collections::HashMap;

use serde::Serialize;

#[derive(Clone, Serialize, Debug)]
pub enum NormalNixExpr {
    Int(i64),
    Float(f64),
    String(String),
    Path(String),
    Var(String),
    Select {
        subject: Box<NormalNixExpr>,
        or_default: Option<Box<NormalNixExpr>>,
        path: Vec<AttrName>,
    },
    OpHasAttr {
        subject: Box<NormalNixExpr>,
        path: Vec<AttrName>,
    },
    Attrs {
        rec: bool,
        attrs: Vec<AttrDef>,
        dynamic_attrs: Vec<DynamicAttrDef>,
    },
    List(Vec<NormalNixExpr>),
    Lambda {
        arg: Option<String>,
        formals: Option<Formals>,
        body: Box<NormalNixExpr>,
    },
    Call {
        fun: Box<NormalNixExpr>,
        args: Vec<NormalNixExpr>,
    },
    Let {
        attrs: Box<NormalNixExpr>,
        body: Box<NormalNixExpr>,
    },
    With {
        attrs: Box<NormalNixExpr>,
        body: Box<NormalNixExpr>,
    },
    If {
        cond: Box<NormalNixExpr>,
        then: Box<NormalNixExpr>,
        else_: Box<NormalNixExpr>,
    },
    Assert {
        cond: Box<NormalNixExpr>,
        body: Box<NormalNixExpr>,
    },
    OpNot(Box<NormalNixExpr>),
    OpEq(Box<NormalNixExpr>, Box<NormalNixExpr>),
    OpNEq(Box<NormalNixExpr>, Box<NormalNixExpr>),
    OpAnd(Box<NormalNixExpr>, Box<NormalNixExpr>),
    OpOr(Box<NormalNixExpr>, Box<NormalNixExpr>),
    OpImpl(Box<NormalNixExpr>, Box<NormalNixExpr>),
    OpUpdate(Box<NormalNixExpr>, Box<NormalNixExpr>),
    OpConcatLists(Box<NormalNixExpr>, Box<NormalNixExpr>),
    OpConcatStrings {
        force_string: bool,
        es: Vec<NormalNixExpr>,
    },
}

#[derive(Clone, Serialize, Debug)]
pub enum AttrName {
    Symbol(String),
    Expr(NormalNixExpr),
}

#[derive(Clone, Serialize, Debug)]
pub struct AttrDef {
    pub name: String,
    pub inherited: bool,
    pub expr: NormalNixExpr,
}

#[derive(Clone, Serialize, Debug)]
pub struct DynamicAttrDef {
    pub name_expr: NormalNixExpr,
    pub value_expr: NormalNixExpr,
}

#[derive(Clone, Serialize, Debug)]
pub struct Formal {
    pub default: Option<NormalNixExpr>,
}

#[derive(Clone, Serialize, Debug)]
pub struct Formals {
    pub ellipsis: bool,
    pub entries: HashMap<String, Formal>,
}
