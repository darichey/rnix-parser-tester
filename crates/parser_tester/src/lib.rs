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
            // Re-serialize the expressions so the keys are in the same order on each side
            let ref_impl_json_str = lhs.to_string();
            let rnix_json_str = rhs.to_string();
            panic!(
                "\n\nref_impl: {ref_impl_json_str}\n\nrnix:     {rnix_json_str}\n\n{}\n\n",
                err
            );
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
        float_e_no_dec_part: "2e01",
        float_e_no_whole_part: ".5e01",
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
        // https://github.com/nix-community/rnix-parser/issues/69
        string_multiline_nested_quotes: indoc!{r#"
            ''
                The "android" ABI is not for 32-bit ARM. Use "androideabi" instead.
            ''
        "#},
        // https://github.com/kamadorueda/alejandra/issues/194
        string_escaped_interpol: r#" ''''\${[1 2]}'' "#,
        path_relative: "foo/bar",
        path_relative_prefixed: "./foo/bar",
        path_relative_cwd: "./.",
        path_absolute: "/foo/bar",
        path_home: "~/foo/bar",
        path_store: "<foo/bar>",
        path_interpolated: r#"d: ./a/b/${"c"}/${d}/e/f"#,
        select: "x: x.y",
        select_nested: "x: x.y.z",
        select_with_default: "x: x.y.z or 37",
        has_attr: "x: x ? y",
        has_attr_compound: "x: x ? y.z",
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
    }
}
