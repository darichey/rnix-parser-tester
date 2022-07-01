use std::env;

use ref_impl_parser::Parser as RefImplParser;
use rnix_to_json::Parser as RnixParser;

fn main() {
    let nix_exprs = [
        "x: ./foo/bar",
        "x: foo/bar/bar",
        "x: /foo/bar",
        "x: ~/foo/bar",
        "x: <foo/bar>",
    ];

    for expr in nix_exprs {
        println!("===========================================");

        println!("{expr}");

        println!("------------");

        let json_str1 = RefImplParser::new().parse(expr);
        println!("{json_str1}");

        let json_str2 = RnixParser::new(".", env::var("HOME").unwrap()).parse(expr);
        println!("{json_str2}");

        println!("------------");

        println!("{}", json_str1 == json_str2);
        println!("===========================================");
    }
}
