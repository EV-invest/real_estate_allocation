//! Hydrated side-by-side of the landing reference widgets vs our live uikit, so
//! the differences are interactive (drag the slider, open the select, tab the
//! button). One component per row, two columns: Reference | Our uikit.
//!
//! The reference column is the landing's real radix markup, sliced out of the
//! persisted section at runtime (so it can't drift); the React widgets can't
//! hydrate here, so that column is static — but it styles against the same
//! `tailwind.css`, which is all the visual comparison needs. The right column is
//! our actual `ev_lib::uikit` components, fully interactive.
//!
//! Serve it: `dx serve --example uikit_compare`
use dioxus::prelude::*;
use ev_lib::uikit::{Button, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Slider};

const REFERENCE: &str = include_str!("portfolio_original.html");

// Mirrors `src/main.rs`: native is the fullstack *server* (calling
// `dioxus::launch` natively boots the web renderer and panics in wasm-bindgen);
// wasm is the client. This app has no backend, so the server just SSRs + serves.
// Embedded so a bare `cargo run` serves it directly: the `asset!` macro only
// resolves to a real URL under `dx`'s asset pipeline, so we sidestep it.
#[cfg(not(target_arch = "wasm32"))]
const CSS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/tailwind.css"));

const HEAD: &str = "border-b border-main-mist/10 pb-2 font-mono text-xs uppercase tracking-wider text-main-mist/60";
#[cfg(not(target_arch = "wasm32"))]
fn main() {
	use dioxus::server::axum::{Router, routing::get};
	// `dioxus::server` serves static assets from `<exe-dir>/public` and panics if
	// it's absent. `dx serve` populates it (and the wasm client, so the page
	// hydrates); a bare `cargo run` never does, so make it exist — the server then
	// SSRs the styled page (no client hydration; use `dx serve --example` for that).
	let exe = std::env::current_exe().expect("exe path");
	std::fs::create_dir_all(exe.parent().expect("exe has parent").join("public")).expect("create public dir");
	dioxus::server::serve(move || async move {
		let css = get(|| async { ([("Content-Type", "text/css; charset=utf-8")], CSS) });
		Ok(Router::new().route("/style.css", css).merge(dioxus::server::router(app)))
	});
}

#[cfg(target_arch = "wasm32")]
fn main() {
	dioxus::launch(app);
}

// Matches the native range to our kit slider: a main-card shaft (h-1.5 ≡
// 0.375rem) filled to `--p` with the accent, a main-mist thumb (size-4 ≡ 1rem)
// that grows a ring/50 on hover/drag like the kit's `hover:ring-4`. Pseudo-element
// selectors can't be Tailwind utilities (this example isn't in the content scan),
// hence raw CSS; colours mirror `tailwind.css`, sizes are rem.
const KIT_RANGE_CSS: &str = "\
.kit-range { -webkit-appearance: none; appearance: none; width: 100%; height: 1rem; background: transparent; cursor: pointer; }
.kit-range::-webkit-slider-runnable-track { height: 0.375rem; border-radius: 9999px; background: linear-gradient(#2a9d8f, #2a9d8f) no-repeat left center / var(--p, 0%) 100%, #0c1626; }
.kit-range::-webkit-slider-thumb { -webkit-appearance: none; appearance: none; height: 1rem; width: 1rem; margin-top: -0.3125rem; border-radius: 9999px; background: #e6e1d3; transition: box-shadow 0.15s ease; }
.kit-range:hover::-webkit-slider-thumb, .kit-range:active::-webkit-slider-thumb { box-shadow: 0 0 0 0.25rem color-mix(in oklab, #2a9d8f 50%, transparent); }
.kit-range::-moz-range-track { height: 0.375rem; border-radius: 9999px; background: #0c1626; }
.kit-range::-moz-range-progress { height: 0.375rem; border-radius: 9999px; background: #2a9d8f; }
.kit-range::-moz-range-thumb { height: 1rem; width: 1rem; border: none; border-radius: 9999px; background: #e6e1d3; transition: box-shadow 0.15s ease; }
.kit-range:hover::-moz-range-thumb, .kit-range:active::-moz-range-thumb { box-shadow: 0 0 0 0.25rem color-mix(in oklab, #2a9d8f 50%, transparent); }";

fn app() -> Element {
	let mut amount = use_signal(|| 100_000.0_f64);
	let mut term = use_signal(|| 5_u32);
	let pct = ((amount() - 50_000.0) / 950_000.0 * 100.0).clamp(0.0, 100.0);

	rsx! {
		document::Link { rel: "stylesheet", r#type: "text/css", href: "/style.css" }
		document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
		document::Link {
			rel: "stylesheet",
			href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Playfair+Display:wght@600;700&display=swap",
		}
		style { dangerous_inner_html: KIT_RANGE_CSS }
		div { class: "dark min-h-screen bg-main-black p-10 font-sans text-white",
			// Shared readout so the three columns are visibly one whole: drag any
			// slider / pick any select and every column reflects the same state.
			div { class: "mb-8 font-mono text-sm text-main-accent-t1",
				"principal ${amount} · term {term}y"
			}
			div {
				class: "grid items-center gap-x-8 gap-y-8",
				style: "grid-template-columns: 120px 320px 320px 320px",

				div {}
				div { class: HEAD, "Reference (landing)" }
				div { class: HEAD, "Our uikit" }
				div { class: HEAD, "Native HTML" }

				Row {
					label: "Slider",
					reference: extract(REFERENCE, "data-slot=\"slider\"", "span"),
					ours: rsx! {
						Slider {
							class: "[&_[data-slot=slider-track]]:bg-main-black/50 [&_[data-slot=slider-range]]:bg-main-accent-t1 [&_[data-slot=slider-thumb]]:border-main-accent-t1",
							min: 50_000.0,
							max: 1_000_000.0,
							step: 10_000.0,
							value: amount(),
							on_value_change: move |v| amount.set(v),
						}
					},
					native: rsx! {
						input {
							r#type: "range",
							class: "kit-range",
							style: "--p: {pct}%",
							min: "50000",
							max: "1000000",
							step: "10000",
							value: "{amount}",
							oninput: move |e| {
								if let Ok(v) = e.value().parse::<f64>() {
									amount.set(v);
								}
							},
						}
					},
				}

				Row {
					label: "Select",
					reference: extract(REFERENCE, "data-slot=\"select-trigger\"", "button"),
					ours: rsx! {
						Select {
							value: term().to_string(),
							on_value_change: move |v: String| { if let Ok(y) = v.parse() { term.set(y); } },
							SelectTrigger { class: "w-full border-main-mist/20 bg-main-black/60 font-mono", SelectValue {} }
							SelectContent {
								for y in [3_u32, 5, 7, 10] {
									SelectItem { value: "{y}", "{y} Years" }
								}
							}
						}
					},
					native: rsx! {
						select {
							class: "w-full rounded-md border border-main-mist/20 bg-main-black/60 px-3 py-2 text-sm",
							onchange: move |e| {
								if let Ok(y) = e.value().parse::<u32>() {
									term.set(y);
								}
							},
							for y in [3_u32, 5, 7, 10] {
								option { value: "{y}", selected: y == term(), "{y} Years" }
							}
						}
					},
				}

				Row {
					label: "Button",
					reference: extract(REFERENCE, "Request advisory", "button"),
					ours: rsx! {
						Button { class: "w-full rounded-none bg-main-accent-t1 py-5 font-mono text-xs uppercase tracking-wider text-main-black hover:bg-main-mist hover:text-main-brand",
							"Request advisory"
						}
					},
					native: rsx! {
						button { r#type: "button", class: "w-full", "Request advisory" }
					},
				}
			}
		}
	}
}

#[component]
fn Row(label: String, reference: String, ours: Element, native: Element) -> Element {
	let cell = "flex min-h-[3.75rem] items-center border border-main-mist/10 bg-main-black/40 p-5";
	rsx! {
		div { class: "font-semibold", "{label}" }
		div { class: cell,
			div { class: "w-full", dangerous_inner_html: reference }
		}
		div { class: cell,
			div { class: "w-full", {ours} }
		}
		div { class: cell,
			div { class: "w-full", {native} }
		}
	}
}

/// Extract the balanced `<tag>…</tag>` element that contains `anchor`.
fn extract(html: &str, anchor: &str, tag: &str) -> String {
	let a = html.find(anchor).unwrap_or_else(|| panic!("anchor {anchor:?} not found"));
	let (open, close) = (format!("<{tag}"), format!("</{tag}>"));
	let start = html[..a].rfind(&open).expect("opening tag before anchor");
	let mut depth = 0_usize;
	let mut pos = start;
	loop {
		let next_close = html[pos..].find(&close).expect("element closes");
		match html[pos..].find(&open) {
			Some(o) if o < next_close => {
				depth += 1;
				pos += o + open.len();
			}
			_ => {
				depth -= 1;
				pos += next_close + close.len();
				if depth == 0 {
					return html[start..pos].to_string();
				}
			}
		}
	}
}
