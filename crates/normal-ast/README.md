# normal-ast
This crate contains the Rust definition of the "normal form" of a Nix expression.

Note that while the JSON produced by [`ref-impl-parser`](../ref-impl-parser/) follows this structure, that crate does not depend on this one, and that crate does not produce values of this type. This may change in the future.