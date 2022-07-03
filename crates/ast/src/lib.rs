use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub enum NixExpr {
    Int(i64),
    Float(f64),
    String(String),
    Path(String),
    Var(String),
    Select {
        subject: Box<NixExpr>,
        or_default: Option<Box<NixExpr>>,
        path: AttrPath,
    },
    OpHasAttr {
        subject: Box<NixExpr>,
        path: AttrPath,
    },
    Attrs {
        rec: bool,
        attrs: HashMap<String, AttrDef>,
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

#[derive(Serialize)]
pub enum AttrName {
    Symbol(String),
    Expr(NixExpr),
}

#[derive(Serialize)]
pub struct AttrPath {
    components: Vec<AttrName>,
}

#[derive(Serialize)]
pub struct AttrDef {
    inherited: bool,
    expr: NixExpr,
}

#[derive(Serialize)]
pub struct Formal {
    name: String,
    default: NixExpr,
}

#[derive(Serialize)]
pub struct Formals {
    ellipsis: bool,
    entries: Vec<Formal>,
}

#[cfg(test)]
mod tests {
    use crate::NixExpr;

    #[test]
    fn test() {
        let value = NixExpr::Assert {
            cond: Box::new(NixExpr::Int(3)),
            body: Box::new(NixExpr::Int(3))
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, "");
    }
}
