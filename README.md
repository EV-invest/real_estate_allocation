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

Target: `inferno_vps_tokyo` (Ubuntu 22.04, 2 vCPU, 2 GiB RAM).
Same box as the landing site which iframes `/embed/overview`.

### Build

```sh
nix build .#image
nix run .#image.copyTo -- oci-archive:/tmp/rea.tar:rea:latest
scp /tmp/rea.tar inferno_vps_tokyo:/tmp/
ssh inferno_vps_tokyo podman load < /tmp/rea.tar
```

Derivation in `flake.nix` — no git deps, `buildRustPackage` with server
feature auto-selected via `cfg(not(target_arch = "wasm32"))` deps.

### Config

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

### Systemd

```sh
systemctl status evinvest-rea
journalctl -u evinvest-rea -f
```

### Caddy

Landing Caddy routes `/embed/*` → `localhost:59079`.
See `landing/INSTALLATION.md`.

### Known gaps

- **No WASM client bundle**: built with `cargo build`, not `dx build --release`.
  SSR works but client-side hydration absent (calculator, dock panels).
  Wire `dx build --release` into the Nix derivation (needs wasm32 target +
  dioxus-cli + tailwind) for full interactivity.
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

