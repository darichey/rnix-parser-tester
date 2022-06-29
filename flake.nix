{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      pkgs = nixpkgs.legacyPackages.x86_64-linux;
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = [
          # Rust dev
          pkgs.rustc
          pkgs.cargo
          pkgs.rust-analyzer
          pkgs.rustfmt

          # C/C++ dev
          pkgs.nix
          pkgs.nix.dev
          pkgs.boost

          # Useful for viewing JSON AST
          pkgs.jq
        ];
      };
    };
}
