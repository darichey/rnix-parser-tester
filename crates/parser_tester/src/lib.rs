#[cfg(test)]
mod integration_tests {
    use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config};
    use indoc::indoc;
    use std::env;

    fn assert_parses_eq(nix_expr: &str) {
        let ref_impl_json_str = ref_impl_parser::Parser::new().parse(nix_expr);
        let rnix_json_str = rnix_to_json::parse(
            nix_expr,
            env::current_dir()
                .unwrap()
                .into_os_string()
                .into_string()
                .unwrap(),
            env::var("HOME").unwrap(),
        );

        let lhs = serde_json::from_str::<serde_json::Value>(&ref_impl_json_str).unwrap();
        let rhs = serde_json::from_str::<serde_json::Value>(&rnix_json_str).unwrap();

        let config = Config::new(CompareMode::Strict);
        if let Err(err) = assert_json_matches_no_panic(&lhs, &rhs, config) {
            panic!("\n\nref_impl: {ref_impl_json_str}\n\nrnix:     {rnix_json_str}\n\n{}\n\n", err);
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

    gen_tests! {
        int: "1",
        float: "3.14",
        string: r#" "hello world" "#,
        string_interpolated: r#" "hello ${"world"} ${123}" "#,
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
        path_relative: "foo/bar",
        path_relative_prefixed: "./foo/bar",
        path_relative_cwd: "./.",
        path_absolute: "/foo/bar",
        path_home: "~/foo/bar",
        path_store: "<foo/bar>",
        path_interpolated: r#"./foo/${"bar"}"#,
        // Many expressions from this point are nested in a lambda to introduce dummy identifiers.
        // This is necessary because the reference nix parser couples parsing and evaluation and
        // will complain about undeclared identifiers at the parsing phase. As long as lambdas
        // parse equally, then this shouldn't affect the outcome of the test
        select: "x: x.y",
        select_nested: "x: x.y.z",
        select_with_default: "x: x.y.z or 37",
        has_attr: "x: x ? y",
        attrs: "{ x = 5; }",
        attrs_multiple: "{ x = 5; y = 3.14; }",
        attrs_nested: "{ x = { y = { z = 5; }; }; }",
        attrs_compound_key: "{ x.y.z = 5; }",
        attrs_complex: r#"{ x = { y = { z = 5; }; }; a.b.c = 3.14; foo = "Bar"; }"#,
        list: r#"[1 "2" (x: 3) 4.5]"#,
        list_empty: "[]",
        lambda: "x: x",
        lambda_nested: "x: y: x",
        call: "f: f 0",
        call_multiple_args: "f: f 0 1 2",
        call_nested: "f: g: f 0 (g 0 1) 2",
        let: "let x = 5; in x",
        let_multiple: "let x = 5; y = 3.14; in x",
        let_compound_key: "let x.y.z = 5; in x",
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
        math_prec: "(0 + 1 + -2 - 3) * -(4 / 5)"
    }
}
