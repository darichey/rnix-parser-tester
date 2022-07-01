use std::{env, fs};

use ref_impl_parser::Parser as RefImplParser;
use rnix_to_json::Parser as RnixParser;

fn main() {
    let nix_exprs = [
        // &fs::read_to_string("test.nix").unwrap()
        "x: x ? y.z.a"
    ];

    for expr in nix_exprs {
        println!("===========================================");

        println!("{expr}");

        println!("------------");

        let json_str1 = RefImplParser::new().parse(expr);
        println!("{json_str1}");

        println!("------------");

        let json_str2 = RnixParser::new(".", env::var("HOME").unwrap()).parse(expr);
        println!("{json_str2}");

        println!("------------");

        println!("{}", json_str1 == json_str2);
        println!("===========================================");
    }
}

// use ref_impl_parser::Parser as RefImplParser;
// use rnix_to_json::Parser as RnixParser;
// use std::{env, error::Error, fs, path::Path};

// fn main() -> Result<(), Box<dyn Error>> {
//     let path = env::var("NIX_PATH")?;
//     let nixpkgs = path
//         .split(':')
//         .find(|s| s.starts_with("nixpkgs="))
//         .ok_or("no store path found")?;

//     println!("Nix store path: {}", nixpkgs);

//     recurse(Path::new(&nixpkgs["nixpkgs=".len()..]))
// }
// fn recurse(path: &Path) -> Result<(), Box<dyn Error>> {
//     if path.metadata()?.is_file() {
//         if path.extension().and_then(|s| s.to_str()) != Some("nix") {
//             return Ok(());
//         }

//         println!("Checking {}", path.display());
//         let nix_expr = fs::read_to_string(path)?;
//         if nix_expr.trim().is_empty() {
//             return Ok(());
//         }

//         let json_str1 = RefImplParser::new().parse(&nix_expr);
//         let json_str2 = RnixParser::new(".", env::var("HOME").unwrap()).parse(&nix_expr);

//         if json_str1 != json_str2 {
//             println!("Parses not equal!");
//             println!("Input:");
//             println!("----------");
//             println!("{}", nix_expr);
//             println!("----------");
//             println!("Reference Impl Parse:");
//             println!("----------");
//             println!("{}", json_str1);
//             println!("----------");
//             println!("rnix-parser Parse:");
//             println!("----------");
//             println!("{}", json_str2);
//             return Err("Parses not equal".into());
//         }

//         return Ok(());
//     } else {
//         for entry in path.read_dir()? {
//             let entry = entry?;
//             if entry.file_type()?.is_symlink() {
//                 continue;
//             }
//             recurse(&entry.path())?;
//         }
//     }
//     Ok(())
// }
