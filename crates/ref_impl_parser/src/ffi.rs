use libc::c_char;

#[repr(C)]
pub(crate) struct Parser {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

extern "C" {
    pub(crate) fn init_parser() -> *const Parser;
    pub(crate) fn destroy_parser(parser: *const Parser);
    pub(crate) fn parse_from_str(
        parser: *const Parser,
        nix_expr: *const c_char,
        ok: *mut bool,
    ) -> *const c_char;
    pub(crate) fn parse_from_file(
        parser: *const Parser,
        file_path: *const c_char,
        ok: *mut bool,
    ) -> *const c_char;
}
