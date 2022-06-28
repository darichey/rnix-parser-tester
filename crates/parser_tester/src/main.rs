use ref_impl_parser::Parser as RefImplParser;

fn main() {
    let parser = RefImplParser::new();
    let nix_expr = "let x = 3; in y: x + y";
    let json_str = parser.parse(nix_expr);
    println!("{json_str}")
}
