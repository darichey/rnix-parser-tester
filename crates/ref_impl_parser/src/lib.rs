use std::{
    ffi::{CStr, CString},
    path::Path,
};

mod ffi;

pub struct Parser {
    ffi_parser: *const ffi::Parser,
}

impl Parser {
    pub fn new() -> Parser {
        let ffi_parser = unsafe { ffi::init_parser() };
        Parser { ffi_parser }
    }

    pub fn parse_from_str<S>(&self, nix_expr: S) -> String
    where
        S: AsRef<str>,
    {
        let nix_expr = CString::new(nix_expr.as_ref()).unwrap();
        let nix_expr = nix_expr.as_ptr();
        unsafe {
            let json_str = ffi::parse_from_str(self.ffi_parser, nix_expr);
            CStr::from_ptr(json_str).to_str().unwrap().to_string()
        }
    }

    pub fn parse_from_file<P>(&self, path: P) -> String
    where
        P: AsRef<Path>,
    {
        let path = CString::new(path.as_ref().display().to_string()).unwrap();
        let path = path.as_ptr();
        unsafe {
            let json_str = ffi::parse_from_file(self.ffi_parser, path);
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
        parser.parse_from_str("bad expression");
    }
}
