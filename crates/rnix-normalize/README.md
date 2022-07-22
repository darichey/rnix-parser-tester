# rnix-normalize
This crate contains the normalization phase, by which ASTs produced by [`rnix-ast`](../rnix-ast/) are converted to the type exported by [`normal-ast`](../normal-ast/).

## Normalization Rules
Each normalization rule describes how to take an `RNixExpr` (the AST produced by rnix-parser) and transform it into its equivalent `NormalNixExpr` (the AST produced by the reference impl). Each rule has a corresponding function in [`lib.rs`](./src/lib.rs).
