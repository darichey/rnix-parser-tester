use ref_impl_parser::Parser as RefImplParser;

fn main() {
    let parser = RefImplParser::new();
    let nix_expr = "3 + (5 - 7)";
    let json_str = parser.parse(nix_expr);
    println!("{json_str}")
}
