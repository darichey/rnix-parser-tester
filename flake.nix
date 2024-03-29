{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [
          (final: prev: {
            nix = prev.nix.overrideAttrs (old: {
              patches = (old.patches or [ ]) ++ [
                ./crates/ref-impl-parser/reference_to_json/patch/combine-string-token.patch
              ];
            });
          })
        ];
      };
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Rust dev
          rustc
          cargo
          rust-analyzer
          rustfmt
          clippy

          # C/C++ dev
          nix
          nix.dev
          boost

          # Useful for viewing JSON AST
          jq
        ];
      };
    };
}
