# Deployment

Target: `inferno_vps_tokyo` (Ubuntu 22.04, 2 vCPU, 2 GiB).
Same box as the landing site which iframes `/embed/overview`.

Shipped as an OCI image (same flow as the landing backend), built locally and
`podman load`ed on the VPS — the box is too weak to build.

## Build & deploy

```sh
nix build .#image
nix run .#image.copyTo -- oci-archive:/tmp/rea.tar:rea:latest
scp /tmp/rea.tar inferno_vps_tokyo:/tmp/
ssh inferno_vps_tokyo 'podman load < /tmp/rea.tar && systemctl restart evinvest-rea'
```

The image is a `nix2container` build: every `/nix/store` path is its own
layer, so glibc/openssl/etc. shared with the landing backend image are stored
once on disk and `podman load` skips layers it already has.

## Config & data

Everything persistent lives in one host dir mounted at `/data` (the image's
`WorkingDir`). The entrypoint takes no baked args — the unit appends
`--config /data/config.toml`. `socket_addr` MUST bind `0.0.0.0` or the port is
unreachable from outside the container.

`/opt/evinvest/rea-data/config.toml`:

```toml
socket_addr = "0.0.0.0:59079"
db_path = "/data/app.db"
data_dir = "/data/properties"
layout_path = "/data/dashboard_layout.json"
admin_token = "change-me-in-prod"
admins = []
maps_api_key = "not-set"
```

## Systemd

`/etc/systemd/system/evinvest-rea.service` — rootless podman, host dir mounted
at `/data`, port published to localhost (Caddy fronts TLS):

```ini
[Service]
ExecStartPre=-/usr/bin/podman rm -f evinvest-rea
ExecStart=/usr/bin/podman run --rm --name evinvest-rea \
  -p 127.0.0.1:59079:59079 \
  -v /opt/evinvest/rea-data:/data \
  rea:latest --config /data/config.toml
ExecStop=/usr/bin/podman stop evinvest-rea
Restart=on-failure
```

```sh
systemctl status evinvest-rea
journalctl -u evinvest-rea -f
```

## Caddy

Landing Caddy routes `/embed/*` → `localhost:59079`.

## Known gaps

- **No WASM client bundle**: built with `cargo build`, not `dx build --release`.
  SSR works but client hydration absent. Wire `dx build --release` into the
  Nix derivation (needs wasm32 target + dioxus-cli + tailwind).
- **maps_api_key** dummy — Maps won't load (dashboard-only, embed unaffected).
