use std::{env, error::Error, fs};

use rnix::types::TypedNode;
use rnix_to_json::{normalize_nix_expr, NixExpr as RnixNixExpr};

// cargo run -p tools -- foo.nix
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let file = &args[1];

    let nix_expr = fs::read_to_string(file)?;

    println!("==== Reference impl json ====");
    let ref_impl_json_str = ref_impl_parser::Parser::new().parse(&nix_expr);
    println!("{}", ref_impl_json_str);

    println!();

    println!("==== rnix-parser AST dump ====");
    let ast = rnix::parse(&nix_expr);
    println!("{}", ast.root().dump());

    println!();

    println!("==== rnix-parser higher level AST ====");
    let ast = RnixNixExpr::try_from(ast)?;
    println!("{:#?}", ast);

    println!();

    println!("==== rnix-parser normalized AST ====");
    let ast = normalize_nix_expr(
        ast,
        env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap(),
        env::var("HOME").unwrap(),
    );
    println!("{:#?}", ast);

    Ok(())
}
