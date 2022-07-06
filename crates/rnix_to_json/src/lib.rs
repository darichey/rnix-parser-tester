use normalize::normalize_nix_expr;

use crate::ast::NixExpr;

mod ast;
mod normalize;

pub fn parse(nix_expr: &str, home_path: String, base_path: String) -> String {
    let nix_expr = rnix::parse(nix_expr);
    let nix_expr = NixExpr::try_from(nix_expr).unwrap();

    // println!("{:#?}", nix_expr);

    let nix_expr = normalize_nix_expr(nix_expr, home_path, base_path);
    serde_json::to_string(&nix_expr).unwrap()
}
