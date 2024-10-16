{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";

    flake-utils = { url = "github:numtide/flake-utils"; };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = { nixpkgs.follows = "nixpkgs"; };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile
          ./rust-toolchain.toml;
        craneLib = ((crane.mkLib pkgs).overrideToolchain rustToolchain);
        src = craneLib.cleanCargoSource ./.;
        nativeBuildInputs = with pkgs; [ rustToolchain pkg-config openssl ];
        buildInputs = [ ];
        commonArgs = { inherit src buildInputs nativeBuildInputs; };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        bin = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
      in with pkgs; {
        packages = {
          inherit bin;
          default = bin;
        };
        devShells.default = mkShell { inputsFrom = [ bin ]; };
      });
}
