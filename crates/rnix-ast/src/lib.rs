pub mod ast;
pub mod convert;

pub fn parse(nix_expr: &str) -> Result<ast::RNixExpr, convert::ToAstError> {
    ast::RNixExpr::try_from(rnix::Root::parse(nix_expr))
}
