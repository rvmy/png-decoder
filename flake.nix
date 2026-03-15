{
  description = "OSDev Rust dev shell";

  inputs = {
    nixpkgs-unstable.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs-unstable,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";

      pkgs-unstable = import nixpkgs-unstable {
        inherit system;
        overlays = [
          (import rust-overlay)
        ];
      };
    in
    {
      devShells.${system}.default = pkgs-unstable.mkShell {
        packages = [
          pkgs-unstable.rust-analyzer
          pkgs-unstable.rust-bin.stable.latest.default
        ];

        shellHook = ''
          export DEV_SHELL=1
         echo "🚀 Dev shell loaded"
          exec fish
        '';
      };
    };
}
