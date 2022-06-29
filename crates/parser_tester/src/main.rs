use ref_impl_parser::Parser as RefImplParser;
use rnix_to_json::nix_expr_to_json as rnix_parse;

fn main() {
    let nix_expr = "3 + (5 - 7)";

    let json_str1 = RefImplParser::new().parse(nix_expr);
    let json_str2 = rnix_parse(nix_expr);

    println!("{json_str1}");
    println!("{json_str2}");
}
