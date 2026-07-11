#![feature(default_field_values)]
//! Native SSR of the Portfolio embed → a self-contained static HTML snapshot, baked by
//! the flake as `portfolio.html` and served by the conductor as the `RemoteElement`
//! fallback (shown until/unless the live wasm bundle upgrades). Rendered from the SAME
//! `view` components the live bundle mounts, so it can't drift.
//!
//! `src/lib.rs` is `#![cfg(target_arch = "wasm32")]` (the cdylib is wasm-only), so `view`
//! is pulled in directly rather than through the crate — it compiles natively here.
//!
//! Build-time inputs stand in for the wasm-only ones: `AssetOrigin("")` (root-relative,
//! the conductor serves `/mfe/seed/...`) and `building = None` (no live fetch → the three
//! featured stats render the standard `MISSING` placeholder). Dark-only; the conductor is
//! dark-only, so there is no light variant.

#[path = "../src/view.rs"]
mod view;

use dioxus::prelude::*;
use view::{AssetOrigin, Featured, Overview};

fn main() {
	let css = std::env::var("SNAPSHOT_CSS")
		.map(|p| std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("SNAPSHOT_CSS={p}: {e}")))
		.expect("SNAPSHOT_CSS must point at the compiled mfe.css (set by the flake installPhase)");

	let mut dom = VirtualDom::new(Snapshot);
	dom.rebuild_in_place();
	let body = dioxus_ssr::render(&dom);

	// Self-contained: a minimal reset + the inlined mfe.css + dark `color-scheme`. No host
	// fonts (mfe.css ships none; system-font fallback is acceptable for a fallback tile).
	print!(
		"<!doctype html>\n<html lang=\"en\"><head>\
<meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<style>:root{{color-scheme:dark}}*{{box-sizing:border-box}}html,body{{margin:0}}</style>\
<style>{css}</style>\
</head><body style=\"background:#070d18\">{body}</body></html>\n"
	);
}
#[component]
fn Snapshot() -> Element {
	use_context_provider(|| AssetOrigin(String::new()));
	rsx! { Overview { building: Featured(None) } }
}
