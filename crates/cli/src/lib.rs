use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config};
use rnix_ast::ast::NixExpr as RNixExpr;
use rnix_normalize::normalize_nix_expr;
use std::{env, error::Error};

pub fn get_ref_impl_json<S>(input: S) -> String
where
    S: AsRef<str>,
{
    ref_impl_parser::Parser::new().parse(input.as_ref())
}

pub fn get_rnix_json<S>(input: S) -> Result<String, Box<dyn Error>>
where
    S: AsRef<str>,
{
    let ast = normalize_nix_expr(
        RNixExpr::try_from(rnix::parse(input.as_ref()))?,
        env::current_dir()?.into_os_string().into_string().unwrap(),
        env::var("HOME")?,
    );
    let json = serde_json::to_string(&ast)?;

    Ok(json)
}

pub fn assert_parses_eq_no_panic<S>(nix_expr: S) -> Result<(), Box<dyn Error>>
where
    S: AsRef<str>,
{
    let ref_impl_json = get_ref_impl_json(&nix_expr);
    let rnix_json = get_rnix_json(&nix_expr)?;

    let lhs = serde_json::from_str::<serde_json::Value>(&ref_impl_json)?;
    let rhs = serde_json::from_str::<serde_json::Value>(&rnix_json)?;

    let config = Config::new(CompareMode::Strict);

    Ok(assert_json_matches_no_panic(&lhs, &rhs, config).map_err(JsonMismatch)?)
}

#[derive(Debug)]
pub struct JsonMismatch(pub String);

impl std::fmt::Display for JsonMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for JsonMismatch {}

#[cfg(test)]
mod integration_tests {
    use crate::assert_parses_eq_no_panic;
    use indoc::indoc;

    fn assert_parses_eq<S>(nix_expr: S)
    where
        S: AsRef<str>,
    {
        if let Err(err) = assert_parses_eq_no_panic(nix_expr) {
            panic!("{err}");
        }
    }

    macro_rules! gen_tests {
        ($($name:ident : $nix:expr),* $(,)?) => {
            $(
                paste::item! {
                    #[test]
                    fn [< test_ $name >]() {
                        assert_parses_eq($nix);
                    }
                }
            )*
        };
    }

    // Many expressions are nested in a lambda to introduce dummy identifiers.
    // This is necessary because the reference nix parser couples parsing and
    // evaluation and will complain about undeclared identifiers at the
    // parsing phase. As long as lambdas parse equally, then this shouldn't
    // affect the outcome of the test.
    gen_tests! {
        int: "1",
        int_leading_zeros: "001",
        float: "3.14",
        float_no_whole_part: ".14",
        float_e: "2.5e01",
        float_e_no_whole_part: ".5e01",
        string: r#" "hello world" "#,
        string_interpolated: r#" "hello ${"world"} ${123}" "#,
        string_escaped_quote: r#" "hello \"world\"" "#,
        string_multiline: indoc!{r#"
            ''
            Hello world
            ''
        "#},
        // FIXME: I don't think this test is working correctly. Shouldn't it fail because of https://github.com/nix-community/rnix-parser/issues/71 ?
        string_multiline_indented: indoc!{r#"
            ''
                foo
                bar
            ''
        "#},
        // https://github.com/nix-community/rnix-parser/issues/69
        string_multiline_nested_quotes: r#" ''"foo"'' "#,
        // https://github.com/kamadorueda/alejandra/issues/194
        string_escaped_interpol: r#" ''''\${[1 2]}'' "#,
        path_relative: "foo/bar",
        path_relative_prefixed: "./foo/bar",
        path_relative_parent: "./foo/..",
        path_relative_cur: "./.",
        path_absolute: "/foo/bar",
        path_absolute_parent: "/foo/bar/..",
        path_absolute_cur: "/foo/bar/.",
        path_home: "~/foo/bar",
        path_home_parent: "~/foo/bar/..",
        path_home_cur: "~/foo/bar/.",
        path_store: "<foo/bar>",
        path_store_parent: "<foo/bar/..>",
        path_store_cur: "<foo/bar/.>",
        path_interpolated: r#"./${"foo"}"#,
        select: "x: x.y",
        select_nested: "x: x.y.z",
        select_with_default: "x: x.y.z or 37",
        select_string: r#" {}."foo" "#,
        select_string_interp: r#" {}."${"foo"}" "#,
        select_dynamic: "x: {}.${x}",
        select_dynamic_constant_string: r#" {}.${"foo"} "#,
        has_attr: "x: x ? y",
        has_attr_compound: "x: x ? y.z",
        has_attr_string: r#" {} ? "foo" "#,
        has_attr_string_interpol: r#" {} ? "${"foo"}" "#,
        has_attr_dynamic: "x: {} ? ${x}",
        has_attr_dynamic_constant_string: r#" {} ? ${"foo"} "#,
        has_attr_select_first_part_not_var: r#" {} ? ${"foo"}.y "#,
        attrs: "{ x = 5; }",
        attrs_multiple: "{ x = 5; y = 3.14; }",
        attrs_nested: "{ x = { y = { z = 5; }; }; }",
        attrs_compound_key: "{ x.y.z = 5; }",
        attrs_rec: "rec { x = 5; y = x; }",
        attrs_dynamic: "x: { ${x} = 5; }",
        attrs_dynamic_constant_string: r#"{ ${"foo"} = "bar"; }"#,
        attrs_dynamic_plain_compound: "x: { ${x}.y = 5; }",
        attrs_dynamic_dynamic_compound: "x: { ${x}.${x} = 5; }",
        attrs_string_key: r#"{ "hello" = "world"; }"#,
        attrs_string_key_interpol: r#"x: { "${x}.y" = 5; }"#,
        attrs_overlapping: r#"{ x.y = "foo"; x.z = "bar"; }"#,
        attrs_inherit: "x: { inherit x; }",
        attrs_inherit_from: "x: { inherit (x) y z; }",
        list: r#"[1 "2" (x: 3) 4.5]"#,
        list_empty: "[]",
        lambda: "x: x",
        lambda_underscore_arg: "_:null",
        lambda_nested: "x: y: x",
        lambda_formals: "{ x }: x",
        lambda_formals_default: "{ x ? null } : x",
        lambda_formals_ellipsis: "{ x, ... }: x",
        lambda_formals_at_left: "inp@{ x }: x",
        lambda_formals_at_right: "{ x }@inp: x",
        call: "f: f 0",
        call_multiple_args: "f: f 0 1 2",
        call_nested: "f: g: f 0 (g 0 1) 2",
        let: "let x = 5; in x",
        let_multiple: "let x = 5; y = 3.14; in x",
        let_compound_key: "let x.y.z = 5; in x",
        let_legacy: "let { x = 5; body = x; }",
        with: "x: with x; y",
        if: "if true then 0 else 1",
        assert: "assert true; 0",
        not: "!true",
        eq: "0 == 1",
        neq: "0 != 1",
        and: "false && true",
        and_assoc: "false && true && false",
        or: "false || true",
        or_assoc: "false || true || false",
        impl: "false -> true",
        impl_assoc: "false -> true -> false",
        update: "{ x = 0; } // { x = 1; }",
        update_assoc: "{ x = 0; } // { x = 1; } // { x = 2; }",
        concat_lists: "[0] ++ [1]",
        concat_lists_assoc: "[0] ++ [1] ++ [2]",
        concat_strings: r#" "hello" + "world" "#,
        concat_strings_assoc: r#" "hello" + "world" + "foo" "#,
        plus: "0 + 1",
        plus_assoc: "0 + 1 + 2",
        minus: "0 - 1",
        minus_assoc: "0 - 1 - 2",
        times: "0 * 1",
        times_assoc: "0 * 1 * 2",
        divide: "0 / 1",
        divide_assoc: "0 / 1 / 2",
        less: "0 < 1",
        less_eq: "0 <= 1",
        greater: "0 > 1",
        greater_eq: "0 >= 1",
        negate: "-5",
        math_prec: "(0 + 1 + -2 - 3) * -(4 / 5)",
        import: "import ./foo.nix",
        or_special_handling: "[1 or 2]",
        // This is a kind of sanity check relating to how the reference impl sorts attr set keys.
        // In particular, it maintains a global set of symbols, and attributes are sorted by when
        // their corresponding symbols were created. However, there are a bunch of built-in symbols
        // which are created earlier than all the others: https://github.com/NixOS/nix/blob/7e23039b7f491f8517309e0c20653d6d80c37dd7/src/libexpr/eval.cc#L426-L462
        // So, without doing anything, `outputs` would appear _before_ `description` in the below set.
        // However, we don't actually care about attribute order in Nix, so to make things easier, we
        // sort lexicographically on key name in both the ref impl and rnix normalization phases.
        // So, this test verifies that both are sorting correctly despite the ref impl's default behavior.
        attr_set_key_sorting: r#"{ description = "foo"; outputs = "bar"; a = "a"; }"#,
        cur_pos: "__curPos",
    }
}

// #[cfg(test)]
// mod nixpkgs_test {
//     use std::{env, fs, path::Path};

//     use crate::assert_parses_eq_no_panic;

//     #[test]
//     #[ignore] // Expensive, so ignored by default
//     fn test() {
//         let path = env::var("NIX_PATH").unwrap();
//         let nixpkgs = path.split(':').find(|s| s.starts_with("nixpkgs=")).unwrap();

//         recurse(Path::new(&nixpkgs["nixpkgs=".len()..]))
//     }

//     fn recurse(path: &Path) {
//         if path.metadata().unwrap().is_file() {
//             if path.extension().and_then(|s| s.to_str()) != Some("nix") {
//                 return;
//             }

//             print!("{} ... ", path.display());
//             let nix_expr = fs::read_to_string(path).unwrap();
//             if nix_expr.trim().is_empty() {
//                 return;
//             }

//             if let Err(err) = assert_parses_eq_no_panic(&nix_expr) {
//                 println!("\x1b[31mFAILED\x1b[0m");
//                 // println!("{err}");
//             } else {
//                 println!("\x1b[32mok\x1b[0m");
//             }
//         } else {
//             for entry in path.read_dir().unwrap() {
//                 let entry = entry.unwrap();
//                 if entry.file_type().unwrap().is_symlink() {
//                     continue;
//                 }
//                 recurse(&entry.path());
//             }
//         }
//     }
// }
