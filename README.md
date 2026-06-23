# real_estate_allocation
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-1.92+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/real_estate_allocation.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/real_estate_allocation)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/real_estate_allocation)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/real_estate_allocation-loc.json)
<br>
[<img alt="ci errors" src="https://img.shields.io/github/actions/workflow/status/valeratrades/real_estate_allocation/errors.yml?branch=main&style=for-the-badge&style=flat-square&label=errors&labelColor=420d09" height="20">](https://github.com/valeratrades/real_estate_allocation/actions?query=branch%3Amain) <!--NB: Won't find it if repo is private-->
[<img alt="ci warnings" src="https://img.shields.io/github/actions/workflow/status/valeratrades/real_estate_allocation/warnings.yml?branch=main&style=for-the-badge&style=flat-square&label=warnings&labelColor=d16002" height="20">](https://github.com/valeratrades/real_estate_allocation/actions?query=branch%3Amain) <!--NB: Won't find it if repo is private-->

Real Estate allocation service and its microfrontend
<!-- markdownlint-disable -->
<details>
<summary>
<h2>Installation</h2>
</summary>

## Deployment

Target: `inferno_vps_tokyo` (Ubuntu 22.04, 2 vCPU, 2 GiB).
Same box as the landing site which iframes `/embed/overview`.

Shipped as an OCI image (same flow as the landing backend), built locally and
`podman load`ed on the VPS — the box is too weak to build.

### Build & deploy

```sh
nix build .#image
nix run .#image.copyTo -- oci-archive:/tmp/rea.tar:rea:latest
scp /tmp/rea.tar inferno_vps_tokyo:/tmp/
ssh inferno_vps_tokyo 'podman load < /tmp/rea.tar && systemctl restart evinvest-rea'
```

The image is a `nix2container` build: every `/nix/store` path is its own
layer, so glibc/openssl/etc. shared with the landing backend image are stored
once on disk and `podman load` skips layers it already has.

### Config & data

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

### Systemd

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

### Caddy

Landing Caddy routes `/embed/*` → `localhost:59079`.

### Known gaps

- **No WASM client bundle**: built with `cargo build`, not `dx build --release`.
  SSR works but client hydration absent. Wire `dx build --release` into the
  Nix derivation (needs wasm32 target + dioxus-cli + tailwind).
- **maps_api_key** dummy — Maps won't load (dashboard-only, embed unaffected).

</details>
<!-- markdownlint-restore -->



<br>

<sup>
	This repository follows <a href="https://github.com/valeratrades/.github/tree/master/best_practices">my best practices</a> and <a href="https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md">Tiger Style</a> (except "proper capitalization for acronyms": (VsrState, not VSRState) and formatting). For project's architecture, see <a href="./docs/ARCHITECTURE.md">ARCHITECTURE.md</a>.
</sup>

#### License

<sup>
	Licensed under <a href="LICENSE">Blue Oak 1.0.0</a>
</sup>

<br>

<sub>
	Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be licensed as above, without any additional terms or conditions.
</sub>

