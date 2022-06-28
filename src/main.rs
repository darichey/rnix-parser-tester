use std::ffi::{CStr, CString};

use libc::c_char;

#[repr(C)]
struct Parser {
    _data: [u8; 0],
    _marker:
        core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

extern "C" {
    fn init_parser() -> *mut Parser;
    fn destroy_parser(parser: *mut Parser);
    fn nix_expr_to_json_str(parser: *mut Parser, nix_expr: *const c_char) -> *const c_char;
}

fn main() {
    let state = unsafe { init_parser() };

    let nix_expr = CString::new("1-1").unwrap();
    let s = unsafe {
        CStr::from_ptr(nix_expr_to_json_str(state, nix_expr.as_ptr())).to_str().unwrap()
    };
    println!("{s}");

    let nix_expr = CString::new("let x = 3; y = 5; in z: x + y + z").unwrap();
    let s = unsafe {
        CStr::from_ptr(nix_expr_to_json_str(state, nix_expr.as_ptr())).to_str().unwrap()
    };
    println!("{s}");

    unsafe { destroy_parser(state) };
}
