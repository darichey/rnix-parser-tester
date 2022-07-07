use std::ffi::{CStr, CString};

use ast::{AttrDef, AttrName, DynamicAttrDef, Formal, Formals, NixExpr};
use libc::c_char;
use ordered_float::NotNan;

#[repr(C)]
pub(crate) struct Parser {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

extern "C" {
    pub(crate) fn init_parser() -> *const Parser;
    pub(crate) fn destroy_parser(parser: *const Parser);
    pub(crate) fn parse_nix_expr(parser: *const Parser, nix_expr: *const c_char) -> *mut NixExpr;
}

fn c_string_to_string(c_str: *const c_char) -> String {
    unsafe { CStr::from_ptr(c_str).to_str().unwrap().to_string() }
}

#[no_mangle]
pub(crate) extern "C" fn mk_int(value: i64) -> *mut NixExpr {
    Box::into_raw(Box::new(NixExpr::Int(value)))
}

#[no_mangle]
pub(crate) extern "C" fn mk_float(value: f64) -> *mut NixExpr {
    Box::into_raw(Box::new(NixExpr::Float(
        NotNan::new(value).expect("NaN is not a valid Nix float value"),
    )))
}

#[no_mangle]
pub(crate) extern "C" fn mk_string(value: *const c_char) -> *mut NixExpr {
    Box::into_raw(Box::new(NixExpr::String(c_string_to_string(value))))
}

#[no_mangle]
pub(crate) extern "C" fn mk_path(value: *const c_char) -> *mut NixExpr {
    Box::into_raw(Box::new(NixExpr::Path(c_string_to_string(value))))
}

#[no_mangle]
pub(crate) extern "C" fn mk_var(value: *const c_char) -> *mut NixExpr {
    Box::into_raw(Box::new(NixExpr::Var(c_string_to_string(value))))
}

#[no_mangle]
pub(crate) extern "C" fn mk_select(
    subject: *mut NixExpr,
    or_default: *mut NixExpr,
    path: *const *const AttrName,
    path_len: usize,
) -> *mut NixExpr {
    unsafe {
        Box::into_raw(Box::new(NixExpr::Select {
            subject: Box::from_raw(subject),
            or_default: if or_default.is_null() {
                None
            } else {
                Some(Box::from_raw(or_default))
            },
            path: (),
        }))
    }
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_has_attr(
    subject: *mut NixExpr,
    path: *const *const AttrName,
    path_len: usize,
) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_attrs(
    rec: bool,
    attrs: *const *const AttrDef,
    attrs_len: usize,
    dynamic_attrs: *const *const DynamicAttrDef,
    dynamic_attrs_len: usize,
) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_list(elems: *const *mut NixExpr, elems_len: usize) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_lambda(
    arg: *const *const c_char,
    formals: *const Formals,
    body: *mut NixExpr,
) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_call(
    fun: *mut NixExpr,
    args: *const *mut NixExpr,
    args_len: usize,
) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_let(attrs: *mut NixExpr, body: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_with(attrs: *mut NixExpr, body: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_if(
    cond: *mut NixExpr,
    then: *mut NixExpr,
    else_: *mut NixExpr,
) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_assert(cond: *mut NixExpr, body: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_not(expr: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_eq(lhs: *mut NixExpr, rhs: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_neq(lhs: *mut NixExpr, rhs: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_and(lhs: *mut NixExpr, rhs: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_or(lhs: *mut NixExpr, rhs: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_impl(lhs: *mut NixExpr, rhs: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_update(lhs: *mut NixExpr, rhs: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_concat_lists(lhs: *mut NixExpr, rhs: *mut NixExpr) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_op_concat_strings(
    force_strings: bool,
    exprs: *const *mut NixExpr,
    exprs_len: usize,
) -> *mut NixExpr {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_attr_name_symbol(symbol: *const c_char) -> *const AttrName {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_attr_name_expr(expr: *mut NixExpr) -> *const AttrName {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_attr_def(
    name: *const c_char,
    inherited: bool,
    expr: *mut NixExpr,
) -> *const AttrDef {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_dynamic_attr_def(
    name_expr: *mut NixExpr,
    value_expr: *mut NixExpr,
) -> *const DynamicAttrDef {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_formal(name: *const c_char, def: *mut NixExpr) -> *const Formal {
    todo!()
}

#[no_mangle]
pub(crate) extern "C" fn mk_formals(
    ellipsis: bool,
    entries: *const *const Formal,
    entries_len: usize,
) -> *const Formals {
    todo!()
}
