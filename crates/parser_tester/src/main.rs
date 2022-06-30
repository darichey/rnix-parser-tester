use ref_impl_parser::Parser as RefImplParser;
use rnix_to_json::nix_expr_to_json as rnix_parse;

fn main() {
    // let nix_expr = r#"let y = "y"; in { x = "hello"; }.x.${y} or "world""#;
    // let nix_expr = r#"{ x.y.z = "hello"; }"#;

    let nix_expr = r#"{ x = 3; }"#;

    let json_str = RefImplParser::new().parse(nix_expr);
    println!("{json_str}");

    let json_str = rnix_parse(nix_expr);
    println!("{json_str}");
}
