use std::ffi::{CStr, CString};

mod ffi;

pub struct Parser {
    ffi_parser: *const ffi::Parser,
}

impl Parser {
    pub fn new() -> Parser {
        let ffi_parser = unsafe { ffi::init_parser() };
        Parser { ffi_parser }
    }

    pub fn parse(&self, nix_expr: &str) -> String {
        let nix_expr = CString::new(nix_expr).unwrap();
        let nix_expr = nix_expr.as_ptr();
        unsafe {
            let json_str = ffi::nix_expr_to_json_str(self.ffi_parser, nix_expr);
            CStr::from_ptr(json_str).to_str().unwrap().to_string()
        }
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        unsafe { ffi::destroy_parser(self.ffi_parser) }
    }
}

#[cfg(test)]
mod reference_to_json_tests {
    use crate::Parser;

    #[test]
    fn test_bad_parse_doesnt_crash() {
        let parser = Parser::new();
        parser.parse("bad expression");
    }
}
