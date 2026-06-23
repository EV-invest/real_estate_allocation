# Deployment

Target: `inferno_vps_tokyo` (Ubuntu 22.04, 2 vCPU, 2 GiB RAM).
Same box as the landing site which iframes `/embed/overview`.

## Build

```sh
nix build .#image
nix run .#image.copyTo -- oci-archive:/tmp/rea.tar:rea:latest
scp /tmp/rea.tar inferno_vps_tokyo:/tmp/
ssh inferno_vps_tokyo podman load < /tmp/rea.tar
```

Derivation in `flake.nix` — no git deps, `buildRustPackage` with server
feature auto-selected via `cfg(not(target_arch = "wasm32"))` deps.

## Config

Minimal `/opt/evinvest/rea/config.toml`:

```toml
socket_addr = "0.0.0.0:59079"
db_path = "/opt/evinvest/rea-data/app.db"
data_dir = "/opt/evinvest/rea-data/properties"
layout_path = "/opt/evinvest/rea-data/dashboard_layout.json"
admin_token = "change-me-in-prod"
admins = []
maps_api_key = "not-set"
```

## Systemd

```sh
systemctl status evinvest-rea
journalctl -u evinvest-rea -f
```

## Caddy

Landing Caddy routes `/embed/*` → `localhost:59079`.
See `landing/INSTALLATION.md`.

## Known gaps

- **No WASM client bundle**: built with `cargo build`, not `dx build --release`.
  SSR works but client-side hydration absent (calculator, dock panels).
  Wire `dx build --release` into the Nix derivation (needs wasm32 target +
  dioxus-cli + tailwind) for full interactivity.
- **maps_api_key** dummy — Maps won't load (dashboard-only, embed unaffected).
