pub mod ast;
pub mod convert;

pub fn parse(nix_expr: &str) -> Result<ast::NixExpr, convert::ToAstError> {
    ast::NixExpr::try_from(rnix::parse(nix_expr))
}
