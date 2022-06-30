use std::fs;

use ref_impl_parser::Parser as RefImplParser;
use rnix_to_json::nix_expr_to_json as rnix_parse;

fn main() {
    // let nix_expr = r#"let y = "y"; in { x = "hello"; }.x.${y} or "world""#;
    // let nix_expr = r#"{ x.y.z = "hello"; }"#;

    // let nix_expr = r#"let f = inputs@{ x, ... }: x; in f"#;
    // let nix_expr = r#"{ nixpkgs }: nixpkgs.legacyPackages.x86_64-linux"#;

    let nix_expr = fs::read_to_string("./flake.nix").unwrap();
    let nix_expr = &nix_expr;

    let json_str1 = RefImplParser::new().parse(nix_expr);
    println!("{json_str1}");

    println!("===========================================");

    let json_str2 = rnix_parse(nix_expr);
    println!("{json_str2}");

    println!("{}", json_str1 == json_str2);
}
