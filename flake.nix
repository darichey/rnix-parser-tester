{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Rust dev
          rustc
          cargo
          rust-analyzer
          rustfmt

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
