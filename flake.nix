{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        devShell =
          with pkgs;
          mkShell.override
            {
              stdenv = stdenvAdapters.useMoldLinker clangStdenv;
            }
            {
              buildInputs = [
                # https://github.com/oxalica/rust-overlay/issues/136#issuecomment-1716902831
                (lib.hiPrio (
                  rust-bin.stable.latest.minimal.override {
                    extensions = [ "rust-docs" ];
                  }
                ))
                (rust-bin.selectLatestNightlyWith (
                  toolchain:
                  toolchain.minimal.override {
                    extensions = [ "rustfmt" ];
                  }
                ))
                cargo-make
                openssl
                pkg-config
                cargo-watch
              ];

              # https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/5?u=fuwn
              RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            };
      }
    );
}
