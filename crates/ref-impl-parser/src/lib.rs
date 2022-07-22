use std::{
    error::Error,
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

    pub fn parse_from_str<S>(&self, nix_expr: S) -> Result<String, Box<dyn Error>>
    where
        S: AsRef<str>,
    {
        let nix_expr = CString::new(nix_expr.as_ref())?;
        let nix_expr = nix_expr.as_ptr();
        unsafe {
            let ok = Box::into_raw(Box::new(false));
            let res = ffi::parse_from_str(self.ffi_parser, nix_expr, ok);
            self.handle_result(res, *Box::from_raw(ok))
        }
    }

    pub fn parse_from_file<P>(&self, path: P) -> Result<String, Box<dyn Error>>
    where
        P: AsRef<Path>,
    {
        let path = CString::new(path.as_ref().display().to_string()).unwrap();
        let path = path.as_ptr();
        unsafe {
            let ok = Box::into_raw(Box::new(false));
            let res = ffi::parse_from_file(self.ffi_parser, path, ok);
            self.handle_result(res, *Box::from_raw(ok))
        }
    }

    unsafe fn handle_result(
        &self,
        json_str: *const i8,
        ok: bool,
    ) -> Result<String, Box<dyn Error>> {
        let res = CStr::from_ptr(json_str).to_str()?.to_string();
        if ok {
            Ok(res)
        } else {
            Err(ReferenceImplError(res))?
        }
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        unsafe { ffi::destroy_parser(self.ffi_parser) }
    }
}

#[derive(Debug)]
struct ReferenceImplError(String);

impl std::fmt::Display for ReferenceImplError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ReferenceImplError {}

#[cfg(test)]
mod reference_to_json_tests {
    use crate::Parser;

    #[test]
    fn test_bad_parse_doesnt_crash() {
        let parser = Parser::new();
        let _ = parser.parse_from_str("bad expression");
    }
}
