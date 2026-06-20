{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    v_flakes.url = "github:valeratrades/v_flakes?ref=v1.6";
    # Brand assets. Not a flake — just a pinned source tree we copy the logo out
    # of. "Latest logo" = `nix flake update ev_assets` (bumps flake.lock).
    ev_assets = { url = "github:EV-invest/assets"; flake = false; };
  };
  outputs = { self, nixpkgs, rust-overlay, flake-utils, pre-commit-hooks, v_flakes, ev_assets }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          allowUnfree = true;
        };
        #NB: can't load rust-bin from nightly.latest, as there are week guarantees of which components will be available on each day.
        rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rust-docs" "rustc-codegen-cranelift-preview" ];
          targets = [ "wasm32-unknown-unknown" ];
        });
        pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
        manifest = (pkgs.lib.importTOML ./real_estate_allocation/Cargo.toml).package;
        pname = manifest.name;
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

        # Brand logo from the pinned `ev_assets` input, copied into the served
        # assets dir (gitignored; declaratively populated, never hand-edited).
        logoSrc = "${ev_assets}/logo/logo.svg";

        rs = v_flakes.rs {
          inherit pkgs rust;
          build = {
            deny = false;
            workspace = let deprecate_by = "v1.0.0"; in {
              "./real_estate_allocation/" = [ "git_version" "log_directives" { deprecate = { by_version = deprecate_by; force = true; }; } ];
            };
          };
        };
        github = v_flakes.github {
          inherit pkgs pname rs;
          enable = true;
          lastSupportedVersion = "nightly-2026-06-16";
          jobs.default = true;
          gitignore.extra = ''
            real_estate_allocation/assets/tokens.css
            real_estate_allocation/assets/logo.svg
          '';
        };
        readme = v_flakes.readme-fw {
          inherit pkgs pname;
          defaults = true;
          lastSupportedVersion = "nightly-1.92";
          rootDir = ./.;
          badges = [ "msrv" "crates_io" "docs_rs" "loc" "ci" ];
        };
        combined = v_flakes.utils.combine { inherit rust; modules = [ rs github readme ]; };

        # ── dev orchestrator ─────────────────────────────────────────────────
        # `nix run .#dev` → Tailwind watch + the fullstack `dx serve`, together.
        # Self-contained (`writeShellApplication` bakes runtimeInputs onto PATH)
        # so it works without first entering the devShell.
        #
        # IMPORTANT: resolve the repo at *runtime* via `git rev-parse`, never
        # `toString ./.` — the latter locks the wrapper to the read-only
        # /nix/store snapshot, where neither cargo (target/) nor npm
        # (node_modules/) can write. Run `nix run .#dev` from anywhere in the repo.
        runDev = pkgs.writeShellApplication {
          name = "run-dev";
          runtimeInputs = with pkgs; [ rust dioxus-cli nodejs git ];
          text = ''
            repo="$(git rev-parse --show-toplevel)"
            cd "$repo/real_estate_allocation"

            cp -f ${logoSrc} ./assets/logo.svg

            # Tailwind v4 standalone CLI needs `tailwindcss` resolvable from
            # node_modules; install once, then build + watch.
            if [ ! -d node_modules ]; then
              npm install
            fi
            npx @tailwindcss/cli -i ./input.css -o ./assets/tailwind.css
            npx @tailwindcss/cli -i ./input.css -o ./assets/tailwind.css --watch & css=$!
            trap 'kill "$css" 2>/dev/null || true' EXIT INT TERM

            # `--interactive false`: dx's default full-screen TUI assumes it owns
            # the terminal and corrupts when it shares stdout with the css watcher.
            cd "$repo"
            exec dx serve --package real_estate_allocation --port 3000 --interactive false
          '';
        };
      in
      {
        # `nix run .#dev` → Tailwind watch + fullstack dx serve (the one command
        # you need for a dev view). `.#default` aliases it.
        apps = {
          dev = { type = "app"; program = "${runDev}/bin/run-dev"; };
          default = { type = "app"; program = "${runDev}/bin/run-dev"; };
        };

        packages =
          let
            rustc = rust;
            cargo = rust;
            rustPlatform = pkgs.makeRustPlatform {
              inherit rustc cargo stdenv;
            };
          in
          {
            default = rustPlatform.buildRustPackage {
              inherit pname;
              version = manifest.version;

              buildInputs = with pkgs; [
                openssl.dev
              ];
              nativeBuildInputs = with pkgs; [ pkg-config ];

              cargoLock.lockFile = ./Cargo.lock;
              src = pkgs.lib.cleanSource ./.;

              # `asset!("/assets/logo.svg")` needs the file present at compile
              # time; the gitignored copy isn't in the pure source, so stage it.
              postPatch = "cp -f ${logoSrc} real_estate_allocation/assets/logo.svg";
            };
          };

        devShells.default =
          with pkgs;
          mkShell {
            inherit stdenv;
            shellHook =
              pre-commit-check.shellHook
              + combined.shellHook
              + ''
                cp -f ${(v_flakes.files.treefmt) { inherit pkgs; }} ./.treefmt.toml
                cp -f ${logoSrc} ./real_estate_allocation/assets/logo.svg
              '';

            packages = [
              mold
              openssl
              pkg-config
              rust
              dioxus-cli # `dx serve` (fullstack dev server)
              nodejs # standalone Tailwind v4 CLI
            ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;

            env.RUST_BACKTRACE = 1;
            env.RUST_LIB_BACKTRACE = 0;
          };
      }
    );
}
