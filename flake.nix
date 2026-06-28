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
        # rust-lld (the wasm32 linker) embeds a bad rpath on macOS — it looks for
        # libLLVM.dylib in bin/../lib but Nix puts it in <rust>/lib, so a wasm build
        # aborts (SIGABRT) at link time. Point the dynamic loader at <rust>/lib.
        # Darwin-only: the var doesn't exist on Linux's loader.
        dyldFallback = pkgs.lib.optionalString pkgs.stdenv.isDarwin
          ''export DYLD_FALLBACK_LIBRARY_PATH="${rust}/lib''${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"'';
        pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
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

        # ── production image builder (the `master` half of the split) ────────
        # `nix run .#buildImage` → the dashboard prod artifact: builds the dx
        # release dashboard (WASM client + manganis-resolved assets), assembles a
        # FROM-scratch image around the dx server binary + its runtime closure, and
        # writes a `podman load`-able archive. The VPS is too weak to build, so:
        # `nix run .#buildImage` → scp → `podman load` → restart. The embed bundle
        # is NOT baked in here — it's `.#embeds`, served by the landing host.
        #
        # Why a `nix run` and not a pure package: `dx build` downloads a
        # version-matched wasm-opt, which the build sandbox forbids. (Making this
        # pure is feasible via `NO_DOWNLOADS=1` + nix's binaryen/wasm-bindgen on
        # PATH — deferred to the unified gitops container interface.)
        #
        # `-C strip=debuginfo` is prod-only (dev `dx serve` keeps full debuginfo):
        # dx forces DWARF into the release wasm, which wasm-opt's binary writer
        # aborts on; stripping both fixes the abort and halves the bundle.
        runBuildImage = pkgs.writeShellApplication {
          name = "build-image";
          runtimeInputs = [ rust pkgs.dioxus-cli wasm-bindgen-cli pkgs.tailwindcss_4 pkgs.binaryen pkgs.git pkgs.coreutils pkgs.zstd pkgs.patchelf ];
          text = ''
            repo="$(git rev-parse --show-toplevel)"
            # `result/` so the multi-GB artifact stays out of the worktree root (it's
            # gitignored). A leftover `nix build` `result` symlink isn't a dir — drop it.
            [ -d "$repo/result" ] || rm -f "$repo/result"
            mkdir -p "$repo/result"
            out="$repo/result/rea-image.tar.zst"

            # sccache's daemon can hold a dead nix-shell's TMPDIR; respawn it clean.
            sccache --stop-server >/dev/null 2>&1 || true

            cp -f ${logoSrc} "$repo/real_estate_allocation/assets/logo.svg"
            ( cd "$repo/real_estate_allocation" && tailwindcss -i ./input.css -o ./assets/tailwind.css )

            echo "▶ building dashboard (dx build --release)…"
            ( cd "$repo/real_estate_allocation"
              export RUSTFLAGS="--cfg tokio_unstable --cfg web_sys_unstable_apis -C strip=debuginfo"
              dx build --release --package real_estate_allocation )

            web="$repo/target/dx/real_estate_allocation/release/web"
            test -x "$web/server"
            test -f "$web/public/index.html"

            ctx="$(mktemp -d)"
            # copied store paths are read-only, so chmod before rm or cleanup errors.
            trap 'chmod -R u+w "$ctx" 2>/dev/null || true; rm -rf "$ctx"' EXIT
            cp -a "$web/server"            "$ctx/server"
            cp -a "$web/public"            "$ctx/public"

            # dx builds the binary in the dev shell, whose glibc/libgcc differ from
            # any sandbox-built base — so overlaying onto one leaves the ELF
            # interpreter missing. Bundle the binary's *actual* runtime closure
            # (interpreter + rpath store paths + cacert for outbound TLS) and build
            # FROM scratch: correct for whatever toolchain dx used, and far smaller
            # than a full base. (printf over heredoc: a nix '''' string bakes its
            # own indentation into a heredoc body/terminator.)
            echo "▶ collecting the dx binary's nix closure…"
            roots="$(patchelf --print-interpreter "$ctx/server"; patchelf --print-rpath "$ctx/server" | tr ':' '\n'; printf '%s\n' ${pkgs.cacert})"
            sp="$(printf '%s\n' "$roots" | grep -oE '/nix/store/[^/]+' | sort -u)"
            mkdir -p "$ctx/nixstore"
            # word-splitting of $sp (newline-separated store paths) is intentional
            # shellcheck disable=SC2046,SC2086
            for p in $(nix-store -qR $sp | sort -u); do cp -a "$p" "$ctx/nixstore/"; done

            printf 'FROM scratch\nCOPY nixstore /nix/store\nCOPY server /bin/real_estate_allocation\nCOPY public /bin/public\nENV SSL_CERT_FILE=%s/etc/ssl/certs/ca-bundle.crt HOME=/data\nWORKDIR /data\nEXPOSE ${reaPort}\nENTRYPOINT ["/bin/real_estate_allocation"]\n' ${pkgs.cacert} > "$ctx/Dockerfile"

            echo "▶ building image (FROM scratch)…"
            podman build -t localhost/rea:latest "$ctx"

            # Stream straight into zstd (no multi-GB intermediate tar). `--long=27`
            # (128 MB window) catches cross-file redundancy gzip's 32 KB can't;
            # `-T0` multithreads it.
            echo "▶ compressing (zstd --long=27)…"
            podman save localhost/rea:latest | zstd -19 -T0 --long=27 -o "$out" -f
            echo "✅ image → $out ($(du -h "$out" | cut -f1))"
            # `docker save` writes the tag into the OCI ref name, so podman load
            # names it `localhost/latest:latest`; retag to what the unit expects.
            echo "   deploy:"
            echo "     scp \"$out\" \''${vps_addr}:/tmp/"
            echo "     ssh \''${vps_addr} 'zstd -dc --long=27 /tmp/rea-image.tar.zst | podman load; podman tag localhost/latest:latest localhost/rea:latest; systemctl restart evinvest-rea'"
          '';
        };
      in
      {
        # `nix run .#dev` → Tailwind watch + fullstack dx serve (the one command
        # you need for a dev view). `.#default` aliases it. `.#buildImage` builds
        # the master (dashboard) prod image → loadable archive. The embed bundle
        # is `nix build .#embeds` (consumed by the landing host).
        apps = {
          dev = { type = "app"; program = "${runDev}/bin/run-dev"; };
          default = { type = "app"; program = "${runDev}/bin/run-dev"; };
          buildImage = { type = "app"; program = "${runBuildImage}/bin/build-image"; };
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
              nativeBuildInputs = with pkgs; [ pkg-config cmake perl mold pkgs.rustPlatform.bindgenHook tailwindcss_4 ];

              cargoLock.lockFile = ./Cargo.lock;
              src = pkgs.lib.cleanSource ./.;

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
              src = pkgs.lib.cleanSource ./.;
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
            # //TEST ─ pure-nix feasibility of the `nix run .#buildImage` flow ──
            # Does in-sandbox what `runBuildImage` does dynamically: `dx build
            # --release` with downloads disabled. `NO_DOWNLOADS=1` flips dx 0.7's
            # `prefer_no_downloads()`, so its `wasm_opt`/`wasm_bindgen` stages
            # resolve binaries via `which` off PATH (nix `binaryen` 129 matches
            # dx's pinned BINARYEN_VERSION; `wasm-bindgen-cli` =0.2.125 passes
            # dx's exact-version check) instead of fetching — the one thing that
            # forced the imperative `nix run`. Vendored cargo (cargoLock) covers
            # the offline crate graph. Output mirrors `reaBin`: server binary +
            # sibling `public/` in one store path (dioxus resolves `public` by the
            # binary's realpath). Marked //TEST throughout so it greps-and-nukes
            # cleanly if dx touches the network somewhere the source-read missed.
            reaDxBuild = rustPlatform.buildRustPackage {
              pname = "${pname}-dx"; # //TEST
              version = manifest.version;
              src = pkgs.lib.cleanSource ./.;
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
                removeReferencesTo # //TEST
              ];

              postPatch = ''
                cp -f ${logoSrc} real_estate_allocation/assets/logo.svg
                ( cd real_estate_allocation && tailwindcss -i ./input.css -o ./assets/tailwind.css )
              '';

              buildPhase = ''
                runHook preBuild
                ${dyldFallback}
                export PORT=${port}
                export NO_DOWNLOADS=1 # //TEST: dx resolves wasm-opt/wasm-bindgen off PATH
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
                # //TEST: the server binary and the wasm bake the nightly std
                # source path (`…/rust-default/…/library/alloc/src/string.rs`) into
                # `#[track_caller]` panic-location strings — pure data never opened
                # at runtime, but nix's reference scanner sees it and dockerTools
                # drags the whole 2.2 GB toolchain into the image. `runBuildImage`
                # dodges this only by hand-collecting the (clean) rpath instead of
                # the nix closure. Scrub just that one path; the glibc/gcc RUNPATH
                # the binary genuinely needs is a different store path, untouched.
                chmod -R u+w "$out/bin"
                remove-references-to -t ${rust} "$out/bin/real_estate_allocation"
                find "$out/bin/public" -name '*.wasm' \
                  -exec remove-references-to -t ${rust} {} +
                runHook postInstall
              '';
              doCheck = false;
            };

            # //TEST ─ pure replacement for the imperative podman/zstd assembly.
            # `dockerTools.buildLayeredImage` is pure + built-in (no nix2container
            # input), and pulls `reaDxBuild`'s closure automatically — so the
            # manual `patchelf`/`nix-store -qR` closure dance in `runBuildImage`
            # isn't needed: the binary is sandbox-built, its rpath is already
            # proper store paths.
            pureContainerFeasibility = pkgs.dockerTools.buildLayeredImage {
              name = "rea"; # //TEST
              tag = "latest";
              contents = [ pkgs.cacert ];
              config = {
                Entrypoint = [ "${reaDxBuild}/bin/real_estate_allocation" ];
                WorkingDir = "/data";
                ExposedPorts = { "${reaPort}/tcp" = { }; };
                Env = [
                  "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
                  "HOME=/data"
                ];
              };
            };
          in
          {
            default = reaBin;
            bin = reaBin;
            embeds = embeds;
            pureContainerFeasibility = pureContainerFeasibility; # //TEST
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
