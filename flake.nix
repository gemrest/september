{
  description = "Simple & Efficient Gemini-to-HTTP Proxy";

  inputs = {
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";

    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      crane,
      flake-utils,
      nixpkgs,
      rust-overlay,
      self,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;

          overlays = [ (import rust-overlay) ];
        };

        craneLib = crane.mkLib pkgs;

        meta = with pkgs.lib; {
          homepage = "https://github.com/gemrest/september";
          description = "Simple & Efficient Gemini-to-HTTP Proxy";
          license = licenses.gpl3Only;
          maintainers = [ maintainers.Fuwn ];
          mainPackage = "september";
          platforms = platforms.linux;
        };

        september = craneLib.buildPackage {
          inherit meta;

          strictDeps = true;

          src = pkgs.lib.cleanSourceWith {
            src = ./.;

            filter =
              path: type:
              builtins.match ".*css$" path != null
              || builtins.match ".*\\.git.*" path != null
              || (craneLib.filterCargoSources path type);
          };
        };
      in
      {
        packages = {
          inherit september;

          default = self.packages.${system}.september;
        };

        apps = {
          september = {
            inherit meta;

            type = "app";
            program = "${self.packages.${system}.september}/bin/september";
          };

          default = self.apps.${system}.september;
        };

        devShells.default =
          with pkgs;
          mkShell.override
            {
              stdenv = stdenvAdapters.useMoldLinker clangStdenv;
            }
            {
              RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

              buildInputs = [
                cargo-make
                openssl
                pkg-config
                cargo-watch

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
              ];
            };
      }
    );
}
