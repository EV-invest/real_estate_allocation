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

fn main() {
	dioxus::launch(app);
}

fn app() -> Element {
	let mut amount = use_signal(|| 100_000.0_f64);
	let mut term = use_signal(|| 5_u32);

	rsx! {
		document::Stylesheet { href: asset!("/assets/tailwind.css") }
		document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
		document::Link {
			rel: "stylesheet",
			href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Playfair+Display:wght@600;700&display=swap",
		}
		div { class: "dark min-h-screen bg-main-black p-10 font-sans text-white",
			div {
				class: "grid items-center gap-x-8 gap-y-8",
				style: "grid-template-columns: 130px 360px 360px",

				div {}
				div { class: "border-b border-main-mist/10 pb-2 font-mono text-xs uppercase tracking-wider text-main-mist/60", "Reference (landing)" }
				div { class: "border-b border-main-mist/10 pb-2 font-mono text-xs uppercase tracking-wider text-main-mist/60", "Our uikit" }

				Row { label: "Slider", reference: extract(REFERENCE, "data-slot=\"slider\"", "span"),
					Slider {
						class: "[&_[data-slot=slider-track]]:bg-main-black/50 [&_[data-slot=slider-range]]:bg-main-accent-t1 [&_[data-slot=slider-thumb]]:border-main-accent-t1",
						min: 50_000.0,
						max: 1_000_000.0,
						step: 10_000.0,
						value: amount(),
						on_value_change: move |v| amount.set(v),
					}
				}

				Row { label: "Select trigger", reference: extract(REFERENCE, "data-slot=\"select-trigger\"", "button"),
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
				}

				Row { label: "Button", reference: extract(REFERENCE, "Request advisory", "button"),
					Button { class: "w-full rounded-none bg-main-accent-t1 py-5 font-mono text-xs uppercase tracking-wider text-main-black hover:bg-main-mist hover:text-main-brand",
						"Request advisory"
					}
				}
			}
		}
	}
}

#[component]
fn Row(label: String, reference: String, children: Element) -> Element {
	rsx! {
		div { class: "font-semibold", "{label}" }
		div { class: "flex min-h-[60px] items-center border border-main-mist/10 bg-main-black/40 p-5",
			div { class: "w-full", dangerous_inner_html: reference }
		}
		div { class: "flex min-h-[60px] items-center border border-main-mist/10 bg-main-black/40 p-5",
			div { class: "w-full", {children} }
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
