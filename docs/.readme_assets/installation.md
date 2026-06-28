# Deployment

Target: `${vps_addr}` (Ubuntu 22.04, 2 vCPU, 2 GiB).
Same box as the landing site which iframes `/embed/overview`.

Shipped as an OCI image (same flow as the landing backend), built locally and
`podman load`ed on the VPS — the box is too weak to build.

## Build & deploy

```sh
nix build .#image
nix run .#image.copyTo -- oci-archive:/tmp/rea.tar:rea:latest
scp /tmp/rea.tar ${vps_addr}:/tmp/
ssh ${vps_addr} 'podman load < /tmp/rea.tar && systemctl restart evinvest-rea'
```

The image is a `nix2container` build: every `/nix/store` path is its own
layer, so glibc/openssl/etc. shared with the landing backend image are stored
once on disk and `podman load` skips layers it already has.

## Config & data

Everything persistent lives in one host dir mounted at `/data` (the image's
`WorkingDir`). The entrypoint takes no baked args — the unit appends
`--config /data/config.toml`. `socket_addr` MUST bind `0.0.0.0` or the port is
unreachable from outside the container.

The config template `deploy/config.toml` holds prod paths and `${VAR}` env
references for secrets (no secrets in the repo). Deploy renders it through
`reasonable_envsubst` — which inlines the referenced env vars from the local
shell — and ships the result:

```sh
REA_ADMIN_TOKEN=… GOOGLE_MAPS_KEY=… \
  reasonable_envsubst "$(cat deploy/config.toml)" \
  | ssh ${vps_addr} 'cat > /opt/evinvest/rea-data/config.toml \
      && chown evinvest:evinvest /opt/evinvest/rea-data/config.toml \
      && systemctl restart evinvest-rea'
```

`GOOGLE_MAPS_KEY` is the same var the local config (`~/.config/real_estate_allocation.nix`)
resolves at runtime via `maps_api_key.env`; here it's inlined at ship time
instead, since the container has no such env.

## Systemd

`/etc/systemd/system/evinvest-rea.service` — root podman, host network (matches
the backend unit), host dir mounted at `/data`. `socket_addr` in the config
binds the port; Caddy fronts TLS and routes `/embed/*` here.

```ini
[Service]
Type=simple
ExecStartPre=-podman rm -f evinvest-rea
ExecStart=podman run --rm --name evinvest-rea \
  --network=host \
  -v /opt/evinvest/rea-data:/data \
  localhost/rea:latest --config /data/config.toml
Restart=on-failure
RestartSec=5
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
