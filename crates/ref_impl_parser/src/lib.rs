use std::ffi::CString;

use ast::NixExpr;

mod ffi;

pub struct Parser {
    ffi_parser: *const ffi::Parser,
}

impl Parser {
    pub fn new() -> Parser {
        let ffi_parser = unsafe { ffi::init_parser() };
        Parser { ffi_parser }
    }

    pub fn parse(&self, nix_expr: &str) -> NixExpr {
        let nix_expr = CString::new(nix_expr).unwrap();
        let nix_expr = nix_expr.as_ptr();
        unsafe { *Box::from_raw(ffi::parse_nix_expr(self.ffi_parser, nix_expr)) }
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        unsafe { ffi::destroy_parser(self.ffi_parser) }
    }
}

#[cfg(test)]
mod tests {
    use ast::NixExpr;

    use crate::Parser;

    #[test]
    fn test_parse_int() {
        assert_eq!(
            Parser::new().parse("7"),
            NixExpr::Int(7)
        );
    }
}
