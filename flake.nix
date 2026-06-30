{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    pre-commit-hooks.inputs.nixpkgs.follows = "nixpkgs";
    v_flakes.url = "github:valeratrades/v_flakes?ref=v1.6";
    v_flakes.inputs.nixpkgs.follows = "nixpkgs";
    v_flakes.inputs.rust-overlay.follows = "rust-overlay";
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
        # rust-lld (the wasm32 linker) embeds a bad rpath on macOS — it looks for
        # libLLVM.dylib in bin/../lib but Nix puts it in <rust>/lib, so a wasm build
        # aborts (SIGABRT) at link time. Point the dynamic loader at <rust>/lib.
        # Darwin-only: the var doesn't exist on Linux's loader.
        dyldFallback = pkgs.lib.optionalString pkgs.stdenv.isDarwin
          ''export DYLD_FALLBACK_LIBRARY_PATH="${rust}/lib''${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"'';
        # v_flakes ships the treefmt hook; we extend it with the same `.#test`
        # derivation (cargo test / insta snapshots). Kept on pre-push, not
        # pre-commit — a full test run per commit is too slow; flip `stages` to
        # ["pre-commit"] if you want it on every commit.
        preCommitBase = v_flakes.files.preCommit { inherit pkgs; };
        pre-commit-check = pre-commit-hooks.lib.${system}.run (preCommitBase // {
          hooks = preCommitBase.hooks // {
            test = {
              enable = true;
              name = "nix run .#test (cargo test / insta snapshots)";
              entry = "${runTest}/bin/run-test";
              pass_filenames = false;
              stages = [ "pre-push" ];
            };
          };
        });
        manifest = (pkgs.lib.importTOML ./real_estate_allocation/Cargo.toml).package;
        pname = manifest.name;
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

        # Landing's dev page port. Baked into the REA build (`option_env!("PORT")`)
        # so the CORS allowlist default tracks it: config `cors_allowed_origins`
        # defaults to `{PORT}` (the page) + `{PORT}+1` (its API origin). Single
        # source for the build paths below; non-flake `cargo build` falls back to
        # this same value in `config.rs`. It collides in name with dioxus' runtime
        # bind `PORT`, but that's overridden at run time (dx owns the address in
        # dev; main.rs forces 59079 in prod), so a build-time value never binds.
        port = "58843";

        # REA's own dev/prod serving port (the bundle origin landing fetches from).
        # Single source for the `dx serve` bind, the healthcheck default, the
        # container's exposed port, and the devShell `REA_PORT` env below.
        reaPort = "59079";

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
          containerRelease = { registry = "ghcr.io/EV-invest"; };
          jobs.default = true;
          gitignore.extra = ''
            **/node_modules/
            real_estate_allocation/assets/tailwind.css
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
        # so it works without first entering the devShell. Serves only the
        # dashboard; the embed bundle is now the landing host's concern
        # (`nix build .#embeds`), no longer served here.
        #
        # IMPORTANT: resolve the repo at *runtime* via `git rev-parse`, never
        # `toString ./.` — the latter locks the wrapper to the read-only
        # /nix/store snapshot, where neither cargo (target/) nor npm
        # (node_modules/) can write. Run `nix run .#dev` from anywhere in the repo.
        runDev = pkgs.writeShellApplication {
          name = "run-dev";
          runtimeInputs = with pkgs; [ rust dioxus-cli tailwindcss_4 git ];
          text = ''
            repo="$(git rev-parse --show-toplevel)"
            cd "$repo/real_estate_allocation"

            # Build-time only (REA CORS default, see `port`); dx overrides the
            # server's runtime bind PORT, so this doesn't affect the served address.
            export PORT=${port}

            cp -f ${logoSrc} ./assets/logo.svg

            tailwindcss -i ./input.css -o ./assets/tailwind.css
            tailwindcss -i ./input.css -o ./assets/tailwind.css --watch & css=$!
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
            echo "  ▶ serving on http://127.0.0.1:${reaPort}"
            # dx builds the dashboard wasm client, which still pulls miette → onig_sys
            # (C); cross-compiling it to wasm needs the devShell's cc-wrapper (the bare
            # app PATH fails on `gnu/stubs-32.h`), so delegate to `nix develop`.
            #
            # Override `.cargo/config.toml`'s native rustflags for dx only (RUSTFLAGS
            # env fully replaces `[target.*].rustflags`, it doesn't merge): keep just
            # the correctness cfgs, drop the speed flags dx can't tolerate. `dx`
            # intercepts linking via its own linker shim, so a `-fuse-ld=mold`
            # link-arg breaks it ("Failed to read link args from file"); and its
            # incremental build ICEs under the parallel front-end (`-Z threads`).
            # Plain `cargo b`/`cargo r`/`nix build` still read the config and keep both.
            export RUSTFLAGS='--cfg tokio_unstable --cfg web_sys_unstable_apis'
            nix develop "$repo" --command dx serve --package real_estate_allocation --port ${reaPort} #--interactive false
          '';
        };

        # ── test suite: cargo test (runs the insta snapshots) ──────────────
        # Delegates to `nix develop` so cargo gets the devShell toolchain (the
        # `.cargo` sccache/cranelift/mold accelerators a bare app PATH lacks),
        # exactly like runDev does for `dx serve`.
        runTest = pkgs.writeShellApplication {
          name = "run-test";
          runtimeInputs = with pkgs; [ git ];
          text = ''
            repo="$(git rev-parse --show-toplevel)"
            echo "▶ cargo test (insta snapshots)"
            exec nix develop "$repo" --command cargo test
          '';
        };

      in
      let
        rustc = rust;
        cargo = rust;
        rustPlatform = pkgs.makeRustPlatform {
          inherit rustc cargo stdenv;
        };
        # `.cargo` holds dev-only accelerators (sccache rustc-wrapper, cranelift,
        # mold) the hermetic sandbox lacks — drop it so the pure build uses nix's
        # own toolchain instead of failing on a missing `sccache` on PATH.
        pureSrc = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: _type: baseNameOf path != ".cargo";
        };
        reaBin = rustPlatform.buildRustPackage {
          inherit pname;
          version = manifest.version;

          buildInputs = with pkgs; [
            openssl.dev
            sqlite
          ];
          nativeBuildInputs = with pkgs; [ pkg-config cmake perl mold pkgs.rustPlatform.bindgenHook tailwindcss_4 ];

          cargoLock.lockFile = ./Cargo.lock;
          src = pureSrc;

          # `asset!()` needs these present at compile time, but both are
          # gitignored generated artifacts absent from the pure source:
          # `logo.svg` is staged from ev_assets, `tailwind.css` is built from
          # `input.css` here — the same offline CLI the dev/mfe paths use, so
          # release CSS can't drift from a stale committed copy.
          postPatch = ''
            cp -f ${logoSrc} real_estate_allocation/assets/logo.svg
            ( cd real_estate_allocation && tailwindcss -i ./input.css -o ./assets/tailwind.css )
          '';
          # Dioxus fullstack looks for `public/` next to its own binary.
          # buildEnv symlinks don't work (binary resolves realpath), so the
          # dir must be in the same store path as the binary.
          postInstall = "mkdir -p $out/bin/public";
          doCheck = false;
        };

        # ── microfrontend bundle (the `embeds` half of the split) ──────────
        # `nix build .#embeds` → the cross-origin MFE bundle as a pure artifact
        # the landing host bakes into its own `public/` and serves same-origin
        # (the way it bakes the whitepaper/blog). The master image no longer
        # carries it. The wasm graph is pure Rust (miette/syntect/onig dropped
        # with the embed's move off the full REA lib), so it builds in the
        # sandbox with no `dx`/wasm-opt download.
        #
        # All boilerplate (custom-element registration, start fn, manifest) is
        # the `ev_lib::mfe!` macro; these steps are the manganis-free packaging
        # `dx` can't do for a cross-origin remote: wasm-bindgen + utilities-only
        # CSS + seed assets. `wasm-bindgen-cli` is pinned to the `wasm-bindgen`
        # crate (=0.2.125); a skew is a hard schema error at bindgen time.
        embeds = rustPlatform.buildRustPackage {
          pname = "${pname}-mfe";
          version = manifest.version;
          src = pureSrc;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ wasm-bindgen-cli pkgs.tailwindcss_4 ];

          # Build only the wasm crate; the default workspace build would pull the
          # native graph. web_sys_unstable_apis: dioxus-web touches unstable
          # web-sys (mirror the host config's native-target cfg for wasm).
          # getrandom_backend: getrandom 0.3 (transitive via ahash) selects its
          # wasm backend by cfg, not just a feature.
          buildPhase = ''
            runHook preBuild
            ${dyldFallback}
            export RUSTFLAGS='--cfg=web_sys_unstable_apis --cfg=getrandom_backend="wasm_js"'
            cargo build -p real_estate_allocation_mfe --target wasm32-unknown-unknown --release --offline
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            name=real_estate_allocation_mfe
            mkdir -p "$out"
            wasm-bindgen --target web --out-dir "$out" --out-name "$name" \
              "target/wasm32-unknown-unknown/release/$name.wasm"

            # wasm-bindgen `--target web` doesn't auto-init; this generic 2-line
            # ESM wrapper is the bundle's registry entrypoint — importing it runs
            # the `start` fn that registers the custom element. (printf, not a
            # heredoc: a heredoc body/terminator would inherit nix-string indent.)
            printf 'import init from "./%s.js";\nawait init();\n' "$name" > "$out/mfe-real-estate-overview.js"

            ( cd real_estate_allocation && tailwindcss -i ./mfe.css -o "$out/mfe.css" )
            cp -r real_estate_allocation/assets/seed "$out/seed"

            # ponytail: mirrors `MFE_MANIFEST` (the macro's const, not readable
            # from a build script). One remote, hand-kept; a multi-remote setup
            # would emit it from a tiny print step into landing's registry.
            printf '%s\n' '{"name":"real-estate.overview","tag":"mfe-real-estate-overview","kind":"component"}' > "$out/mfe.json"
            runHook postInstall
          '';
          doCheck = false;
        };
        # Server binary + sibling public/, wrapped into `.#container`.
        # `NO_DOWNLOADS=1` flips dx 0.7's `prefer_no_downloads()`, so its
        # `wasm_opt`/`wasm_bindgen` stages resolve binaries via `which` off
        # PATH (nix `binaryen` 129 matches dx's pinned BINARYEN_VERSION;
        # `wasm-bindgen-cli` =0.2.125 passes dx's exact-version check) instead
        # of fetching — what used to force an imperative `nix run`. Vendored
        # cargo (cargoLock) covers the offline crate graph. Output mirrors
        # `reaBin`: server binary + sibling `public/` in one store path
        # (dioxus resolves `public` by the binary's realpath). The embed
        # bundle is NOT baked in — that's `.#embeds`, served by landing.
        reaDxBuild = rustPlatform.buildRustPackage {
          pname = "${pname}-dx";
          version = manifest.version;
          src = pureSrc;
          cargoLock.lockFile = ./Cargo.lock;

          buildInputs = with pkgs; [ openssl.dev sqlite ];
          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            perl
            mold
            pkgs.rustPlatform.bindgenHook
            dioxus-cli
            wasm-bindgen-cli
            binaryen
            tailwindcss_4
            removeReferencesTo
          ];

          postPatch = ''
            cp -f ${logoSrc} real_estate_allocation/assets/logo.svg
            ( cd real_estate_allocation && tailwindcss -i ./input.css -o ./assets/tailwind.css )
          '';

          # `-C strip=debuginfo` is prod-only: dx forces DWARF into the release
          # wasm, which wasm-opt's binary writer aborts on; stripping both fixes
          # the abort and halves the bundle.
          buildPhase = ''
            runHook preBuild
            ${dyldFallback}
            export PORT=${port}
            export NO_DOWNLOADS=1
            export RUSTFLAGS="--cfg tokio_unstable --cfg web_sys_unstable_apis -C strip=debuginfo"
            ( cd real_estate_allocation && dx build --release --package real_estate_allocation )
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            web="target/dx/real_estate_allocation/release/web"
            test -x "$web/server"
            test -f "$web/public/index.html"
            mkdir -p "$out/bin"
            cp -a "$web/server" "$out/bin/real_estate_allocation"
            cp -a "$web/public" "$out/bin/public"
            # The server binary and the wasm bake the nightly std source path
            # (`…/rust-default/…/library/alloc/src/string.rs`) into `#[track_caller]`
            # panic-location strings — pure data never opened at runtime, but
            # nix's reference scanner sees it and dockerTools would drag the whole
            # 2.2 GB toolchain into the image. Scrub just that one path; the
            # glibc/gcc RUNPATH the binary genuinely needs is a different store
            # path, untouched. (Drops the image from ~2.4 GB to ~106 MB.)
            chmod -R u+w "$out/bin"
            remove-references-to -t ${rust} "$out/bin/real_estate_allocation"
            find "$out/bin/public" -name '*.wasm' \
              -exec remove-references-to -t ${rust} {} +
            runHook postInstall
          '';
          doCheck = false;
        };

        # reaDxBuild's closure is pulled via the Entrypoint store-path ref.
        # Secret-free prod config baked into the image; its `.env` fields pull the
        # two secrets (GOOGLE_MAPS_KEY, REA_ADMIN_TOKEN) from the container env
        # that gitops' k8s Secret injects (`envFrom`). Authored in nix and
        # evaluated to JSON here — the container has no `nix`, so the binary reads
        # the baked result, not the `.nix`. Without an explicit `--config` the
        # binary searches only XDG dirs + the prefixed `REAL_ESTATE_ALLOCATION_*`
        # env namespace — neither sees the bare-named Secret vars nor the `/data`
        # prod paths, so it would silently boot on dev defaults (empty maps key,
        # 127.0.0.1 bind that fails the k8s probe). This is what makes it correct.
        prodConfig = pkgs.writeText "config.json" (builtins.toJSON (import ./deploy/config.nix));
        containerStd = v_flakes.container.implement {
          inherit pkgs pname;
          containers."" = {
            port = pkgs.lib.toInt reaPort;
            mounts = [ "/data" ];
            healthPath = "/health";
            criticality = "normal";
            entrypoint = [ "${reaDxBuild}/bin/real_estate_allocation" "--config" "${prodConfig}" ];
            workingDir = "/data";
            imageEnv = [ "HOME=/data" ];
          };
        };
      in
      {
        # `nix run .#dev`  → Tailwind watch + fullstack `dx serve`
        # `nix run .#test` → cargo test (insta snapshots)
        apps = {
          dev = { type = "app"; program = "${runDev}/bin/run-dev"; };
          default = { type = "app"; program = "${runDev}/bin/run-dev"; };
          test = { type = "app"; program = "${runTest}/bin/run-test"; };
        };

        packages = {
          default = reaBin;
          bin = reaBin;
          embeds = embeds;
        } // containerStd.packages;

        containers = containerStd.containers;

        devShells.default =
          with pkgs;
          mkShell {
            inherit stdenv;
            shellHook =
              ''
                if [ "$(nix config show lazy-trees 2>/dev/null)" != true ]; then
                  printf '%s\n' \
                    "✘ This repo requires Determinate Nix with lazy-trees=true." \
                    "  Stock nix produces flake.lock NAR hashes that diverge from CI (private inputs fail to verify)." \
                    "  Install: https://determinate.systems/nix   NixOS: nix.settings.lazy-trees = true" >&2
                  exit 1
                fi
              ''
              + pre-commit-check.shellHook
              + combined.shellHook
              + ''
                cp -f ${(v_flakes.files.treefmt) { inherit pkgs; }} ./.treefmt.toml
                cp -f ${logoSrc} ./real_estate_allocation/assets/logo.svg
                ${dyldFallback}
              '';

            packages = [
              mold
              openssl
              pkg-config
              rust
              dioxus-cli # `dx serve` (fullstack dev server)
              tailwindcss_4 # standalone Tailwind v4 CLI (input.css + mfe.css)
              wasm-bindgen-cli # manual embed builds — must match wasm-bindgen =0.2.125
            ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;

            env.RUST_BACKTRACE = 1;
            env.RUST_LIB_BACKTRACE = 0;
            # Baked into the REA build → CORS allowlist default (see the `port`
            # let-binding). `nix build .#embeds` inherits this via the sandbox.
            env.PORT = port;
            # REA's serving port, for a manual `dx serve --port $REA_PORT` in the shell.
            env.REA_PORT = reaPort;
          };
      }
    );
}
