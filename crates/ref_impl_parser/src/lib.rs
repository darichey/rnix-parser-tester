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
mod tests {
    use crate::Parser;

    #[test]
    fn test_parse() {
        let parser = Parser::new();
        let nix_expr = "let x = 3; in y: x + y";
        let json_str = parser.parse(nix_expr);

        assert_eq!(
            r#"{"attrs":{"attrs":{"x":[false,{"type":"Int","value":3}]},"dynamic_attrs":[],"rec":false,"type":"Attrs"},"body":{"arg":"y","body":{"es":[{"type":"Var","value":"x"},{"type":"Var","value":"y"}],"force_string":false,"type":"ConcatStrings"},"formals":null,"name":"","type":"Lambda"},"type":"Let"}"#,
            json_str
        );
    }
}
