# https://github.com/cargo2nix/cargo2nix/blob/release-0.11.0/flake.nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=release-23.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = inputs: with inputs;

    flake-utils.lib.eachDefaultSystem (system:

      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ cargo2nix.overlays.default ];
        };

        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustChannel = "nightly";
          rustVersion = "2024-05-23"; # "2024-05-23"
          packageFun = import ./Cargo.nix;

          extraRustComponents = ["rustfmt" "clippy"];

          # packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
          #   (pkgs.rustBuilder.rustLib.makeOverride {
          #     name = "alsa-sys";
          #     overrideAttrs = drv: {
          #       propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
          #         pkgs.alsa-lib
          #       ];
          #     };
          #   })
          # ];
        };

        workspaceShell = rustPkgs.workspaceShell {
          # This adds cargo2nix to the project shell via the cargo2nix flake
          # packages = [
          #   cargo2nix.packages."${system}".cargo2nix
          # ];

          CARGO_HTTP_MULTIPLEXING = "false";
          # CARGO_HOME = ./.cargo;
          # RUSTUP_HOME = ./.cargo;
        };

        bootstrapShell = pkgs.mkShell {
          packages = [ cargo2nix.packages."${system}".cargo2nix ];
          nativeBuildInputs = cargo2nix.packages."${system}".cargo2nix.nativeBuildInputs;
        };


      in rec {

        devShells = {
          default = workspaceShell; # nix develop
          bootstrap = bootstrapShell; # nix develop .#bootstrap
        };

        # the packages in `nix build .#packages.<system>.<name>`
        packages = {
          # nix build .#unixsocks
          # nix build .#packages.x86_64-linux.unixsocks
          coordinator = (rustPkgs.workspace.coordinator {}).bin;
          leader = (rustPkgs.workspace.leader {}).bin;
          worker = (rustPkgs.workspace.worker {}).bin;
          # nix build
          default = packages.worker;
        };
      }
    );
}
