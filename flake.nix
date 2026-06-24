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
    nix2container = { url = "github:nlewo/nix2container"; inputs.nixpkgs.follows = "nixpkgs"; };
  };
  outputs = { self, nixpkgs, rust-overlay, flake-utils, pre-commit-hooks, v_flakes, ev_assets, nix2container }:
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

        # Pinned to match the workspace's `wasm-bindgen` (`=0.2.125`) — nixpkgs
        # ships a different minor, and a CLI/crate schema skew is a hard error at
        # `wasm-bindgen` time. Shadows `pkgs.wasm-bindgen-cli` (a `let` binding
        # wins over `with pkgs;`) wherever it's referenced below.
        wasm-bindgen-cli =
          let
            src = pkgs.fetchCrate {
              pname = "wasm-bindgen-cli";
              version = "0.2.125";
              hash = "sha256-zRawtjxMOdTMX+mZaiNR3YYfTiZJhf9qj7kXSSeMxrc=";
            };
          in
          pkgs.buildWasmBindgenCli {
            inherit src;
            cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
              inherit src;
              inherit (src) pname version;
              hash = "sha256-aZCfgR23Qb0Pn4Mm4ToMtuuRQqSJjXCR9li/VvP5CTM=";
            };
          };

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

            # No `exec`: keep this shell as parent so the trap above reaps
            # tailwind on exit. `--interactive false` stops dx from detaching
            # into its own session (TUI mode does setsid) — that detachment is
            # what let it survive fish's ctrl-c and orphan the server holding
            # the port.
            cd "$repo"
            # Reap any fullstack server orphaned by a previous run (dx doesn't
            # always propagate SIGINT to its spawned server child, leaving a
            # `server-<hash>` binary holding the port).
            pkill -f 'target/dx/real_estate_allocation/.*/server-' 2>/dev/null || true
            echo "  ▶ serving on http://127.0.0.1:59079"
            dx serve --package real_estate_allocation --port 59079 #--interactive false
          '';
        };

        # ── microfrontend bundle builder ─────────────────────────────────────
        # `nix run .#mfe` → build the cross-origin MFE bundle into `$REPO/mfe-dist`,
        # which the dev `dx serve` (above) serves at `/mfe` (config `mfe_dir`). All
        # boilerplate (custom-element registration, start fn, manifest) is the
        # `ev_lib::mfe!` macro; these steps are the manganis-free packaging `dx`
        # can't do for a cross-origin remote: wasm-bindgen + utilities-only CSS +
        # seed assets, all under the bundle's own origin.
        #
        # `wasm-bindgen-cli` is pinned to the `wasm-bindgen` crate (`=0.2.125`); a
        # version skew is a hard schema error at bindgen time (see the let-binding).
        mfeSteps = pkgs.writeShellScript "build-mfe-steps" ''
          set -eu
          out="$REPO/mfe-dist"
          name="real_estate_allocation_mfe"

          cd "$REPO"
          # dioxus-web touches some unstable web-sys APIs; the host config sets this
          # cfg for the native target only, so mirror it for wasm here.
          export RUSTFLAGS="--cfg=web_sys_unstable_apis"
          cargo build -p "$name" --target wasm32-unknown-unknown --release

          rm -rf "$out"; mkdir -p "$out"
          wasm-bindgen --target web --out-dir "$out" --out-name "$name" \
            "target/wasm32-unknown-unknown/release/$name.wasm"

          # wasm-bindgen `--target web` doesn't auto-init; this generic 2-line ESM
          # wrapper is the bundle's registry entrypoint — importing it runs the
          # `start` fn that registers the custom element. No contract knowledge.
          # (printf, not a heredoc: a heredoc body/terminator would inherit this
          # nix string's indentation.)
          printf 'import init from "./%s.js";\nawait init();\n' "$name" > "$out/mfe-real-estate-overview.js"

          cd "$REPO/real_estate_allocation"
          [ -d node_modules ] || npm install
          npx @tailwindcss/cli -i ./mfe.css -o "$out/mfe.css"
          cp -r ./assets/seed "$out/seed"

          # ponytail: mirrors `MFE_MANIFEST` (the macro's const, not host-readable
          # from a shell). One remote, hand-kept; a multi-remote setup would emit it
          # from a tiny print step and feed it into landing's registry.
          printf '%s\n' '{"name":"real-estate.overview","tag":"mfe-real-estate-overview","kind":"component"}' > "$out/mfe.json"
          echo "  ▶ MFE bundle built → $out"
        '';
        # Run the steps *inside* the devShell: onig_sys (C, pulled by miette→syntect)
        # cross-compiles to wasm only with the cc-wrapper the devShell sets up — a
        # bare `writeShellApplication` PATH lacks it (fails on `gnu/stubs-32.h`). The
        # devShell also carries `rust`, the pinned `wasm-bindgen-cli`, and `nodejs`.
        runMfe = pkgs.writeShellApplication {
          name = "build-mfe";
          runtimeInputs = with pkgs; [ git ];
          text = ''
            REPO="$(git rev-parse --show-toplevel)"
            export REPO
            exec nix develop "$REPO" --command ${mfeSteps}
          '';
        };
      in
      {
        # `nix run .#dev` → Tailwind watch + fullstack dx serve (the one command
        # you need for a dev view). `.#default` aliases it. `.#mfe` builds the
        # microfrontend bundle dev `dx serve` serves at `/mfe`.
        apps = {
          dev = { type = "app"; program = "${runDev}/bin/run-dev"; };
          default = { type = "app"; program = "${runDev}/bin/run-dev"; };
          mfe = { type = "app"; program = "${runMfe}/bin/build-mfe"; };
        };

        packages =
          let
            rustc = rust;
            cargo = rust;
            rustPlatform = pkgs.makeRustPlatform {
              inherit rustc cargo stdenv;
            };
            reaBin = rustPlatform.buildRustPackage {
              inherit pname;
              version = manifest.version;

              buildInputs = with pkgs; [
                openssl.dev
                sqlite
              ];
              nativeBuildInputs = with pkgs; [ pkg-config cmake perl mold pkgs.rustPlatform.bindgenHook ];

              cargoLock.lockFile = ./Cargo.lock;
              src = pkgs.lib.cleanSource ./.;

              # `asset!("/assets/logo.svg")` needs the file present at compile
              # time; the gitignored copy isn't in the pure source, so stage it.
              postPatch = "cp -f ${logoSrc} real_estate_allocation/assets/logo.svg";
              # Dioxus fullstack looks for `public/` next to its own binary.
              # buildEnv symlinks don't work (binary resolves realpath), so the
              # dir must be in the same store path as the binary.
              postInstall = "mkdir -p $out/bin/public";
              doCheck = false;
            };
            n2c = nix2container.packages.${system}.nix2container;
            # Dioxus fullstack panics without a `public/` directory next to the
            # binary. Production usually generates this via `dx build --release`
            # (WASM client + index.html); until that is wired into the Nix build
            # (needs wasm32 target + dioxus-cli), an empty dir lets SSR work.
            reaImage = n2c.buildImage {
              name = "rea";
              tag = "latest";
              copyToRoot = pkgs.buildEnv {
                name = "image-root";
                paths = with pkgs; [ reaBin cacert sqlite ];
                pathsToLink = [ "/bin" "/lib" "/etc" ];
              };
              config = {
                # No args baked in: deploy appends `--config /data/config.toml`.
                # That config's `socket_addr` is what actually binds — main.rs
                # derives dioxus' IP/PORT env from it, so setting them here would
                # just get overwritten. The config MUST bind 0.0.0.0 to be
                # reachable from outside the container.
                Entrypoint = [ "/bin/real_estate_allocation" ];
                # HOME: v_utils' tracing logs to the XDG state dir, which panics
                # (HomeMissing) without it. Point it at the writable /data volume.
                Env = [
                  "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
                  "HOME=/data"
                ];
                # /data is a mounted volume: config.toml + sqlite db + properties.
                WorkingDir = "/data";
                ExposedPorts = { "59079/tcp" = { }; };
              };
            };
          in
          {
            default = reaBin;
            bin = reaBin;
            image = reaImage;
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
              wasm-bindgen-cli # `nix run .#mfe` — must match wasm-bindgen =0.2.125
            ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;

            env.RUST_BACKTRACE = 1;
            env.RUST_LIB_BACKTRACE = 0;
          };
      }
    );
}
