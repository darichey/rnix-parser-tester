use std::ffi::{CStr, CString};

use libc::c_char;

extern "C" {
    fn nix_expr_to_json_str(nix_expr: *const c_char) -> *const c_char;
}

fn main() {
    let nix_expr = CString::new("1-1").unwrap();
    let s = unsafe {
        CStr::from_ptr(nix_expr_to_json_str(nix_expr.as_ptr())).to_str().unwrap()
    };
    println!("{s}");
}
