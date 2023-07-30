{
  description = "titlelist";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    naersk,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
      naersk' = pkgs.callPackage naersk {};
    in {
      packages.default = naersk'.buildPackage {
        src = ./.;
      };
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          rustc
          cargo
        ];
      };
    });
}
