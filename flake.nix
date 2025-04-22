{
  description = "markdoll is a structured and extensible markup language";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain(p: p.rust-bin.nightly.latest.default);

        # Common arguments can be set here to avoid repeating them later
        # Note: changes here will rebuild all dependency crates
        commonArgs = {
          src = let
            # Only keeps markdown files
            markdownFilter = path: _type: builtins.match ".*md$" path != null;
            markdownOrCargo = path: type: (markdownFilter path type) || (craneLib.filterCargoSources path type);
          in pkgs.lib.cleanSourceWith {
            src = ./.; # The original, unfiltered source
            filter = markdownOrCargo;
            name = "source"; # Be reproducible, regardless of the directory name
          };
          
          strictDeps = true;

          buildInputs = [
            # Add additional build inputs here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
        };

        markdoll = pkgs.lib.customisation.makeOverridable ({ danger ? false, }: 
            craneLib.buildPackage (commonArgs // {
                cargoArtifacts = craneLib.buildDepsOnly commonArgs;
                cargoExtraArgs = pkgs.lib.optionalString danger "--features danger";
            })
        ) { danger = false; };
      in
      {
        checks = {
          inherit markdoll;
        };

        packages = {
          inherit markdoll;
          default = markdoll;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = markdoll;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};
        };
      });
}
