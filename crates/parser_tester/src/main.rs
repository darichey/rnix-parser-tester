use std::{env, fs};

use assert_json_diff::assert_json_eq;
// use ref_impl_parser::Parser as RefImplParser;
use rnix_to_json::parse as rnix_parse;

fn main() {
    let nix_exprs = [&fs::read_to_string("./flake.nix").unwrap()];

    for expr in nix_exprs {
        println!("===========================================");

        println!("{expr}");

        println!("------------");

        // let json_str1 = RefImplParser::new().parse(expr);
        // println!("{json_str1}");

        println!("------------");

        let json_str2 = rnix_parse(
            expr,
            env::current_dir()
                .unwrap()
                .into_os_string()
                .into_string()
                .unwrap(),
            env::var("HOME").unwrap(),
        );
        println!("{json_str2}");

        println!("------------");

        // assert_json_eq!(
        //     serde_json::from_str::<serde_json::Value>(&json_str1).unwrap(),
        //     serde_json::from_str::<serde_json::Value>(&json_str2).unwrap()
        // );
        // println!("ASTs are equivalent!");
        println!("===========================================");
    }
}
