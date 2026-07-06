#![feature(default_field_values)]
#![cfg(target_arch = "wasm32")] // web-sys/gloo-net + `ev_lib::mfe` — wasm-only bundle
//! Marketing surface and cross-origin microfrontend bundle. A standalone port of the landing "Premium Asset
//! Portfolio" bento section — no app shell, so the landing host composes `<tag>`
//! directly into its page. Static content mirroring the landing source; the only
//! interactive tile is the self-contained correlation / risk-premia terminal.

use dioxus::prelude::*;
use ev_lib::{mfe::bundle_origin, uikit::Container};
use real_estate_allocation_core::{domain::Building, factors::profile};

// The producer entrypoint: generates the custom-element registration, the
// `wasm-bindgen(start)` entrypoint, the origin self-derivation, and `MFE_MANIFEST`
// (emitted by the build as `mfe.json`).
ev_lib::mfe! {
	service: "real-estate", name: "overview", kind: component,
	root: crate::Overview, stylesheet: "mfe.css"
}

// Two origins. Banners are static assets served alongside the bundle, so they
// resolve against `bundle_origin()` (wherever the `.js` loaded from — the landing
// host once it bakes the bundle into its own `public/`). The data fetch and the
// dashboard breakout links target REA's *backend*, which is a different origin
// once landing serves the bundle — `rea_origin()` reads it from the host's
// `<meta name="rea-url">`, falling back to `bundle_origin()` (the bundle served by
// REA itself, e.g. standalone). Both tiles are real listings.
// Banners ride a fixed `/mfe/seed/...` path under a 4h CDN TTL, so a stale copy
// (e.g. the past LFS-pointer bug) outlives a rebuild. `?v=` gives a fresh cache
// key — bump on any banner-bytes change. ponytail: manual; a content hash would
// auto-bust if this churns.
const ASSET_V: &str = "2";
const Q1_BANNER: &str = "seed/q1_tower/render.jpg";
const Q1_PROPERTY: &str = "b41510ef-1e74-4d4f-a15c-1dfafdd0ee5a";
const TMS_BANNER: &str = "seed/tms/building.jpg";
const TMS_PROPERTY: &str = "c19bded1-1a13-49ad-a0f0-549b2aec2d0e";

// Swap-fraction slider bounds, in percent (0–100% of the host book moved into S).
const A_MIN: f64 = 0.0;
const A_MAX: f64 = 100.0;
const A_STEP: f64 = 1.0;

// Factor-exposure bounds/step, in percent — the one source for the bar clamps, the
// aria range, and the stepper props (they'd contradict each other if they drifted).
const EXPO_MIN: f64 = 0.0;
const EXPO_MAX: f64 = 100.0;
const EXPO_STEP: f64 = 1.0;

#[component]
pub fn Overview() -> Element {
	rsx! {
		section { id: "portfolio", class: "relative border-t border-main-mist/10 py-24",
			Container {
				// Section header
				div { class: "mb-16 flex flex-col justify-between md:flex-row md:items-end",
					div {
						span { class: "mb-3 block font-mono text-xs uppercase tracking-[0.3em] text-main-accent-t1",
							"Investment Scope"
						}
						h2 { class: "font-serif text-3xl font-light text-white sm:text-5xl",
							"Premium Asset "
							span { class: "font-serif italic text-main-accent-t1", "Portfolio" }
						}
					}
					p { class: "mt-4 max-w-md font-light text-sm text-main-mist/70 md:mt-0",
						"Curated, premium, high-yield developments across Quy Nhon city, focusing on high appreciation seaside villas and urban luxury residences."
					}
				}

				// The old filter tabs were cosmetic-only (never wired to data) — dropped in
				// PR #18: dead controls read as broken, and the taller calculator rebalanced
				// the section without them. Deep filtering lives on the dashboard.

				// Bento grid
				div { class: "grid grid-cols-1 gap-6 md:grid-cols-3",
					FeaturedCard {}
					SideCard {}
					WhyCard {}
					Calculator {}
				}
			}
		}
	}
}
/// REA backend origin for the data fetch + dashboard breakout links. The host page
/// advertises it via `<meta name="rea-url">` (its `NEXT_PUBLIC_REA_URL`); absent it,
/// fall back to the bundle's own origin (REA serving the bundle itself). A plain DOM
/// read — `web-sys` is already linked by dioxus-web, so this costs the bundle nothing.
fn rea_origin() -> String {
	web_sys::window()
		.and_then(|w| w.document())
		.and_then(|d| d.query_selector("meta[name=\"rea-url\"]").ok().flatten())
		.and_then(|m| m.get_attribute("content"))
		.filter(|s| !s.is_empty())
		.unwrap_or_else(bundle_origin)
}

/// Large featured tile (spans two columns). Links to the Q1 Tower property page.
#[component]
fn FeaturedCard() -> Element {
	// Pull Q1's live figures so the headline stats track the DB rather than hard-coded
	// copy. The server enriches the building with `price_series` (the basis for
	// `appreciation_yoy`) before serializing — wire-compatible with `core::domain::Building`.
	let building = use_resource(|| async move {
		let url = format!("{}/api/embed/building/{}", rea_origin(), Q1_PROPERTY);
		let b: Option<Building> = gloo_net::http::Request::get(&url).send().await.ok()?.json().await.ok()?;
		b
	});
	// "-" is reserved for appreciation (genuinely unknown until a year of prices exists).
	// A missing yield or status is a data fault, not an empty value — trace it loudly and
	// render "ERR" so it can't pass for a real figure.
	let guard = building.read();
	let (target_yield, appreciation, status) = match &*guard {
		None => (String::new(), "-".to_string(), String::new()), // still loading
		Some(None) => {
			dioxus::logger::tracing::error!(property = Q1_PROPERTY, "featured card: Q1 Tower failed to resolve");
			("ERR".to_string(), "-".to_string(), "ERR".to_string())
		}
		Some(Some(b)) => {
			let target_yield = if b.target_appreciation > 0.0 {
				format!("{:.1}% p.a.", b.target_appreciation)
			} else {
				dioxus::logger::tracing::error!(property = Q1_PROPERTY, "featured card: Q1 Tower has no target yield set");
				"ERR".to_string()
			};
			let appreciation = match b.appreciation_yoy() {
				Some(p) => format!("{p:+.1}% YoY"),
				None => "-".to_string(),
			};
			let status = match b.state_kinds().next() {
				Some(k) => k.to_string(),
				None => {
					dioxus::logger::tracing::error!(property = Q1_PROPERTY, "featured card: Q1 Tower has no portfolio status");
					"ERR".to_string()
				}
			};
			(target_yield, appreciation, status)
		}
	};
	let origin = bundle_origin();
	let rea = rea_origin();

	rsx! {
		a {
			href: "{rea}/?building={Q1_PROPERTY}",
			// no-underline: UA link styling propagates from the wrapping <a> to every
			// descendant on preflight-less hosts (and children can't undo it).
			class: "group relative flex min-h-[450px] flex-col justify-end overflow-hidden border border-main-mist/10 bg-main-black/40 no-underline md:col-span-2",
			div {
				class: "absolute inset-0 z-0 bg-cover bg-center transition-transform duration-700 group-hover:scale-105",
				style: "background-image: linear-gradient(to top, rgba(7,13,24,0.96) 10%, rgba(7,13,24,0.2)), url({origin}/mfe/{Q1_BANNER}?v={ASSET_V})",
			}
			div { class: "absolute right-4 top-4 bg-main-accent-t1 px-3 py-1.5 font-mono text-[0.625rem] font-bold uppercase tracking-widest text-main-black",
				"Featured Deal"
			}
			div { class: "relative z-10 p-8",
				div { class: "mb-3 flex items-center gap-2 font-mono text-xs text-main-accent-t1",
					IconPin {}
					"Quy Nhơn Beachfront" // TODO: exact street address not in `place.json`
				}
				h3 { class: "mb-4 font-serif text-2xl text-white sm:text-3xl", "Q1 Tower Quy Nhơn" }
				p { class: "mb-6 max-w-xl font-light text-sm text-main-mist/70",
					"Landmark twin-tower beachfront residences rising over Quy Nhơn's crescent bay — a lighthouse-inspired icon pairing five-star resort amenities with panoramic East Sea views."
				}
				div { class: "grid max-w-md grid-cols-3 gap-4 border-t border-main-mist/10 pt-6",
					Stat { label: "Target Yield", value_class: "text-main-accent-t2", "{target_yield}" }
					Stat { label: "Appreciation", value_class: "text-main-accent-t3", "{appreciation}" }
					Stat { label: "Status", value_class: "text-white", "{status}" }
				}
			}
		}
	}
}

/// Standard side tile. Links to the TMS Luxury Hotel & Residence property page.
#[component]
fn SideCard() -> Element {
	let origin = bundle_origin();
	let rea = rea_origin();
	rsx! {
		a {
			href: "{rea}/?building={TMS_PROPERTY}",
			class: "group relative flex min-h-[450px] flex-col justify-end overflow-hidden border border-main-mist/10 bg-main-black/40 no-underline",
			div {
				class: "absolute inset-0 z-0 bg-cover bg-center transition-transform duration-700 group-hover:scale-105",
				style: "background-image: linear-gradient(to top, rgba(7,13,24,0.96) 20%, rgba(7,13,24,0.4)), url({origin}/mfe/{TMS_BANNER}?v={ASSET_V})",
			}
			div { class: "relative z-10 p-8",
				div { class: "mb-3 flex items-center gap-2 font-mono text-xs text-main-accent-t1",
					IconPin {}
					"28 Nguyễn Huệ, Quy Nhơn"
				}
				h3 { class: "mb-4 font-serif text-xl text-white sm:text-2xl", "TMS Luxury Hotel & Residence" }
				p { class: "mb-6 font-light text-sm text-main-mist/70",
					"Quy Nhơn's tallest landmark — a 42-floor beachfront tower pairing five-star Grand Hyams hotel service with branded condotel residences steps from the city beach."
				}
				div { class: "flex items-center justify-between border-t border-main-mist/10 pt-6",
					div {
						span { class: "mb-0.5 block font-mono text-[0.5625rem] uppercase text-main-mist/40", "Avg. Apartment" }
						span { class: "text-sm font-serif font-bold text-white", "$76,000" }
					}
					span { class: "flex items-center font-mono text-xs tracking-wider text-main-accent-t1 transition-colors group-hover:text-white",
						"View Property"
						IconArrow {}
					}
				}
			}
		}
	}
}

/// Static market-context tile (no deep-link).
#[component]
fn WhyCard() -> Element {
	rsx! {
		div { class: "flex flex-col justify-between border border-main-mist/10 bg-main-card p-8",
			div {
				div { class: "mb-6 inline-flex items-center gap-1.5 border border-main-accent-t1/20 bg-main-accent-t1/10 px-2 py-1 font-mono text-[0.5625rem] uppercase tracking-wider text-main-accent-t1",
					IconTrend {}
					"Market Growth"
				}
				h3 { class: "mb-4 font-serif text-xl text-white sm:text-2xl", "Why Quy Nhon?" }
				p { class: "mb-6 font-light text-sm text-main-mist/70",
					"Positioned as the new gateway of Central Vietnam, Quy Nhon is undergoing a multi-billion dollar infrastructure upgrade, transforming into a global science and beach tourism destination."
				}
			}
			ul { class: "space-y-3 border-t border-main-mist/10 pt-6 font-mono text-xs",
				Row { label: "Infrastructure Investment:", value_class: "text-white", "$2.4 Billion" }
				Row { label: "Tourism Growth Rate:", value_class: "text-main-accent-t2", "+28% YoY" }
				Row { label: "FDI Inflow (2025):", value_class: "text-main-accent-t2", "$420M" }
			}
		}
	}
}

// The slider below is hand-inlined from the uikit `Slider`'s compiled markup (colours
// applied directly, not via arbitrary-variant overrides), so the track fill and round
// thumb don't depend on `cn!` merge survival.

fn snap(v: f64) -> f64 {
	let v = v.clamp(A_MIN, A_MAX);
	(((v - A_MIN) / A_STEP).round() * A_STEP + A_MIN).clamp(A_MIN, A_MAX)
}

/// Capture the pointer on the event's real target so a drag keeps tracking after the
/// pointer leaves the element — the browser routes every subsequent move to the captor
/// until pointerup/pointercancel. Without it, leaving the track aborted the drag (#16).
fn capture_pointer(e: &PointerEvent) {
	use dioxus::web::WebEventExt;
	use wasm_bindgen::JsCast;
	let raw = e.data().as_web_event();
	if let Some(el) = raw.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
		let _ = el.set_pointer_capture(raw.pointer_id());
	}
}

/// Correlation / risk-premia terminal (spans two columns). Shows our instrument's
/// correlation profile vs the popular alpha factors and, under probabilistic-Kelly
/// sizing (γ=1), what swapping `w%` of a host book into us does to its effective risk
/// premia (risk cost = σ²/2) and compound performance. See `crate::factors`.
///
/// Layout per the issue-16 Figma (Main → real_estate page): intro + swap slider
/// beside the output panel, then the factor mixer — one row per factor with the
/// exposure drawn as a draggable bar, so the book's composition reads at a glance.
#[component]
fn Calculator() -> Element {
	let p = profile();
	// One exposure signal per factor + the host's current YoY return, all in percent.
	// ponytail: factor count is fixed (`profile()` is constant), so these per-factor
	// hooks keep a stable order across renders.
	let exposures: Vec<Signal<f64>> = p.factors.iter().map(|f| use_signal(|| f.default_exposure * 100.0)).collect();
	let yoy = use_signal(|| 10.0_f64);
	let mut swap = use_signal(|| 0.0_f64);

	// Slider drag state: the track's measured origin/width on the x-axis.
	let mut track = use_signal(|| Option::<std::rc::Rc<MountedData>>::None);
	let mut bounds = use_signal(|| (0.0_f64, 1.0_f64));
	let mut dragging = use_signal(|| false);
	let pct = ((swap() - A_MIN) / (A_MAX - A_MIN) * 100.0).clamp(0.0, 100.0);

	let exp: Vec<f64> = exposures.iter().map(|s| s() / 100.0).collect();
	let out = p.evaluate(&exp, yoy() / 100.0, swap() / 100.0);
	let total: f64 = exposures.iter().map(|s| s()).sum();
	// Σ checksum state: exposures are meant to describe a whole book, so drifting
	// off 100% gets a loud gold chip (t3 = the warning-adjacent tier we have).
	let off_by = total - 100.0;
	let balanced = off_by.abs() < 0.5;

	rsx! {
		div { id: "calculator", class: "flex flex-col gap-6 border border-main-mist/10 bg-main-card p-6 md:col-span-2",
			// Intro + swap slider | output panel
			div { class: "grid grid-cols-1 gap-6 md:grid-cols-[minmax(0,1fr)_20rem]",
				div { class: "flex flex-col gap-5",
					div { class: "flex flex-col gap-2",
						span { class: "font-mono text-[0.6875rem] font-semibold uppercase tracking-[0.22em] text-main-accent-t1", "Risk Terminal" }
						h3 { class: "font-serif text-[1.375rem] text-white", "Correlation Profile" }
						p { class: "font-light text-[0.8125rem] leading-relaxed text-main-mist/70",
							"We are judged on our marginal effect on your book — accretive because we are nearly uncorrelated with the alpha factors you already own."
						}
					}
					div { class: "mt-auto flex flex-col gap-2 font-mono",
						div { class: "flex items-center justify-between",
							label { class: "text-[0.625rem] uppercase tracking-wider text-main-mist/40", "Allocation swapped into Vietnam" }
							span { class: "text-[0.8125rem] font-bold text-main-accent-t1", "{swap():.0}%" }
						}
						span {
							class: "relative flex h-6 w-full touch-none select-none items-center",
							onpointerdown: move |e: PointerEvent| async move {
								let Some(t) = track() else { return };
								capture_pointer(&e);
								let Ok(rect) = t.get_client_rect().await else { return };
								bounds.set((rect.origin.x, rect.size.width));
								dragging.set(true);
								let ratio = (e.client_coordinates().x - rect.origin.x) / rect.size.width.max(f64::EPSILON);
								swap.set(snap(A_MIN + ratio * (A_MAX - A_MIN)));
							},
							onpointermove: move |e: PointerEvent| {
								if !dragging() { return; }
								let (ox, w) = bounds();
								let ratio = (e.client_coordinates().x - ox) / w.max(f64::EPSILON);
								swap.set(snap(A_MIN + ratio * (A_MAX - A_MIN)));
							},
							onpointerup: move |_| dragging.set(false),
							onpointercancel: move |_| dragging.set(false),
							span {
								class: "relative h-1.5 w-full grow overflow-hidden rounded-full bg-main-black/55",
								onmounted: move |e: MountedEvent| track.set(Some(e.data())),
								span {
									class: "absolute h-full bg-main-accent-t1",
									style: "width: {pct}%;",
								}
							}
							span {
								class: "block size-4 shrink-0 rounded-full border border-main-accent-t1 bg-white shadow-sm",
								style: "position: absolute; left: {pct}%; top: 50%; transform: translate(-50%, -50%);",
								role: "slider",
								tabindex: "0",
								"aria-valuenow": swap(),
								"aria-valuemin": A_MIN,
								"aria-valuemax": A_MAX,
								onkeydown: move |e: KeyboardEvent| {
									let next = match e.key() {
										Key::ArrowRight | Key::ArrowUp => swap() + A_STEP,
										Key::ArrowLeft | Key::ArrowDown => swap() - A_STEP,
										Key::Home => A_MIN,
										Key::End => A_MAX,
										_ => return,
									};
									e.prevent_default();
									swap.set(snap(next));
								},
							}
						}
						div { class: "flex justify-between text-[0.625rem] text-main-mist/30",
							span { "0%" }
							span { "25%" }
							span { "50%" }
							span { "75%" }
							span { "100%" }
						}
					}
				}

				// Output panel
				div { class: "flex flex-col gap-3 border border-main-mist/10 bg-main-surface p-5",
					span { class: "font-mono text-[0.625rem] uppercase tracking-wider text-main-mist/40", "Δ Effective Risk Premia" }
					span { class: "font-serif text-3xl font-bold text-main-accent-t3", "{out.delta_risk_premia * 10_000.0:+.1} bps" }
					div { class: "border-t border-main-mist/10" }
					div { class: "flex gap-7 font-mono",
						div { class: "flex flex-col gap-1",
							span { class: "text-[0.625rem] uppercase tracking-wider text-main-mist/40", "Δ Expected Perf" }
							span { class: "text-[0.8125rem] font-bold text-main-accent-t2", "{out.delta_performance * 100.0:+.2}%" }
						}
						div { class: "flex flex-col gap-1",
							span { class: "text-[0.625rem] uppercase tracking-wider text-main-mist/40", "ρ (S, P)" }
							span { class: "text-[0.8125rem] font-bold text-white", "{out.rho_sp:+.2}" }
						}
					}
					p { class: "mt-auto font-light text-[0.625rem] leading-snug text-main-mist/30",
						"*Correlation figures indicative placeholders. Risk cost under probabilistic-Kelly (γ≈1). Actual results may vary."
					}
				}
			}

			// Factor mixer — one row per factor: label · ρ · draggable exposure bar · stepper.
			div { class: "flex flex-col gap-3 font-mono",
				div { class: "flex items-center justify-between",
					span { class: "text-[0.625rem] font-semibold uppercase tracking-[0.15em] text-main-mist/40", "Factor Exposures" }
					div { class: "flex items-center gap-2",
						span {
							class: if balanced { "text-[0.5625rem] uppercase tracking-wider text-main-mist/55" } else { "rounded border border-main-accent-t3/50 bg-main-accent-t3/10 px-1.5 py-0.5 text-[0.6875rem] font-bold uppercase tracking-wider text-main-accent-t3" },
							title: "{off_by:+.0}% vs 100%",
							"Σ {total:.0}%"
						}
						span { class: "text-[0.5625rem] uppercase tracking-wider text-main-mist/30", "drag bars · type · ↑↓" }
					}
				}
				div { class: "flex flex-col gap-2",
					for (f, w) in p.factors.iter().zip(exposures.iter().copied()) {
						FactorRow { label: f.label, rho: f.rho, value: w }
					}
				}
				div { class: "border-t border-main-mist/10" }
				// Host book input — teal-marked: it's the user's own number, not a factor weight.
				div { class: "flex items-center gap-3",
					span { class: "h-3.5 w-[3px] shrink-0 rounded-sm bg-main-accent-t1" }
					span { class: "flex-1 text-[0.625rem] uppercase tracking-wide text-main-mist/70 sm:flex-none", "Host YoY return" }
					span { class: "hidden text-[0.5625rem] uppercase tracking-wider text-main-mist/30 sm:block sm:flex-1",
						"your book's current return — not a factor weight"
					}
					ValueStepper { value: yoy, step: 0.5, big_step: 10.0, min: -50.0, max: 100.0, suffix: "%", accent: true }
				}
			}
		}
	}
}

/// One mixer row: factor label, read-only ρ badge (our correlation to that factor),
/// and the exposure drawn as a draggable bar — length = weight, same interaction
/// contract as the swap slider — plus the standardized `ValueStepper`. On mobile the
/// bar wraps to its own line below the label/ρ/stepper meta line.
#[component]
fn FactorRow(label: &'static str, rho: f64, value: Signal<f64>) -> Element {
	let mut value = value;
	let mut track = use_signal(|| Option::<std::rc::Rc<MountedData>>::None);
	let mut bounds = use_signal(|| (0.0_f64, 1.0_f64));
	let mut dragging = use_signal(|| false);
	let pct = (value() - EXPO_MIN) / (EXPO_MAX - EXPO_MIN) * 100.0;

	rsx! {
		div { class: "grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-x-3 gap-y-1.5 md:grid-cols-[11.25rem_2.5rem_minmax(0,1fr)_auto]",
			span { class: "truncate text-[0.625rem] uppercase tracking-wide text-main-mist/70", {label} }
			span {
				class: if rho >= 0.0 { "text-right text-[0.625rem] text-main-accent-t2" } else { "text-right text-[0.625rem] text-main-accent-t3" },
				"{rho:+.2}"
			}
			span {
				class: "relative max-md:order-last max-md:col-span-full flex h-4 cursor-ew-resize touch-none select-none items-center",
				role: "slider",
				tabindex: "0",
				"aria-label": "{label} exposure",
				"aria-valuenow": value(),
				"aria-valuemin": EXPO_MIN,
				"aria-valuemax": EXPO_MAX,
				onpointerdown: move |e: PointerEvent| async move {
					let Some(t) = track() else { return };
					capture_pointer(&e);
					let Ok(rect) = t.get_client_rect().await else { return };
					bounds.set((rect.origin.x, rect.size.width));
					dragging.set(true);
					let ratio = (e.client_coordinates().x - rect.origin.x) / rect.size.width.max(f64::EPSILON);
					value.set((EXPO_MIN + ratio * (EXPO_MAX - EXPO_MIN)).round().clamp(EXPO_MIN, EXPO_MAX));
				},
				onpointermove: move |e: PointerEvent| {
					if !dragging() { return; }
					let (ox, w) = bounds();
					let ratio = (e.client_coordinates().x - ox) / w.max(f64::EPSILON);
					value.set((EXPO_MIN + ratio * (EXPO_MAX - EXPO_MIN)).round().clamp(EXPO_MIN, EXPO_MAX));
				},
				onpointerup: move |_| dragging.set(false),
				onpointercancel: move |_| dragging.set(false),
				onkeydown: move |e: KeyboardEvent| {
					let next = match e.key() {
						Key::ArrowRight | Key::ArrowUp => value() + EXPO_STEP,
						Key::ArrowLeft | Key::ArrowDown => value() - EXPO_STEP,
						Key::Home => EXPO_MIN,
						Key::End => EXPO_MAX,
						_ => return,
					};
					e.prevent_default();
					value.set(next.clamp(EXPO_MIN, EXPO_MAX));
				},
				span {
					class: "relative h-1.5 w-full overflow-hidden rounded-full bg-main-black/55",
					onmounted: move |e: MountedEvent| track.set(Some(e.data())),
					span {
						class: "absolute h-full rounded-full bg-main-accent-t1",
						style: "width: {pct}%;",
					}
				}
				span {
					class: "pointer-events-none absolute block size-2.5 rounded-full border border-main-accent-t1 bg-white shadow-sm",
					style: "left: {pct}%; top: 50%; transform: translate(-50%, -50%);",
				}
			}
			ValueStepper { value, step: EXPO_STEP, big_step: 10.0, min: EXPO_MIN, max: EXPO_MAX, suffix: "%" }
		}
	}
}

/// Standardized numeric input (the uikit primitive #16 asks for — upstream to
/// `ev_lib::uikit` once 0.5 opens): a segmented `[ +10 | −10 | input ]` box.
/// Buttons step by `big_step` clamped to `[min, max]`; the input auto-resizes to
/// its content (ch-based), ↑/↓ nudge by `step`, Enter commits, Esc reverts, blur
/// commits-or-restores. `accent` marks user-owned inputs with the teal border.
#[component]
fn ValueStepper(value: Signal<f64>, step: f64, big_step: f64, min: f64, max: f64, #[props(default)] suffix: &'static str, #[props(default)] accent: bool) -> Element {
	let mut value = value;
	// `Some` holds the raw buffer while the user types; `None` shows the formatted
	// value — so external writers (bars, buttons) reflect instantly unless mid-edit.
	let mut editing = use_signal(|| Option::<String>::None);
	let display = editing().unwrap_or_else(|| format!("{:.1}{suffix}", value()));
	// +3, not +1: the box is border-box, so its `ch` width must also swallow the
		// pl-3/pr-2 padding — a tighter buffer clips the value against the left divider.
		let width_ch = display.chars().count().max(4) + 3;
	let parse_raw = move |raw: &str| raw.trim().trim_end_matches(suffix).trim().parse::<f64>().ok();
	let mut commit = move |raw: String| {
		if let Some(v) = parse_raw(&raw) {
			value.set(v.clamp(min, max));
		}
		editing.set(None);
	};
	// Step from what's on screen: an uncommitted typed buffer wins over the last
	// committed value, so ↑ after typing "8" gives 9 — not committed+step.
	let mut nudge = move |delta: f64| {
		let base = editing().as_deref().and_then(parse_raw).unwrap_or_else(|| value());
		value.set((base + delta).clamp(min, max));
		editing.set(None);
	};

	// Host preflight (unlayered or in a layer above `reamfe`) sets `color`/`font:
	// inherit` on button/input, beating our layered utilities on those elements.
	// So: color/size live on the container and inner spans (preflight never touches
	// those), and the form controls just inherit them.
	rsx! {
		div {
			class: if accent { "flex h-[1.375rem] shrink-0 items-stretch overflow-hidden rounded border border-main-accent-t1/40 bg-main-black/40 font-mono text-xs text-white" } else { "flex h-[1.375rem] shrink-0 items-stretch overflow-hidden rounded border border-main-mist/20 bg-main-black/40 font-mono text-xs text-white" },
			button {
				r#type: "button",
				class: "group/btn flex cursor-pointer appearance-none border-0 bg-transparent p-0",
				onclick: move |_| nudge(big_step),
				span { class: "flex h-full items-center px-2 text-[0.625rem] font-semibold text-main-mist/55 transition-colors group-hover/btn:bg-main-mist/5 group-hover/btn:text-white",
					"+{big_step:.0}"
				}
			}
			span { class: "w-px shrink-0 bg-main-mist/20" }
			button {
				r#type: "button",
				class: "group/btn flex cursor-pointer appearance-none border-0 bg-transparent p-0",
				onclick: move |_| nudge(-big_step),
				span { class: "flex h-full items-center px-2 text-[0.625rem] font-semibold text-main-mist/55 transition-colors group-hover/btn:bg-main-mist/5 group-hover/btn:text-white",
					"−{big_step:.0}"
				}
			}
			span { class: "w-px shrink-0 bg-main-mist/20" }
			input {
				r#type: "text",
				inputmode: "decimal",
				class: "appearance-none border-0 bg-transparent pl-3 pr-2 text-right text-xs text-white outline-none",
				style: "width: {width_ch}ch;",
				value: display,
				oninput: move |e: FormEvent| editing.set(Some(e.value())),
				// Seed with the shortest round-trip repr, not the .1-rounded display —
				// a bare focus+blur must never mutate a value typed at finer precision.
				onfocus: move |_| editing.set(Some(value().to_string())),
				onblur: move |_| {
					if let Some(raw) = editing() { commit(raw); }
				},
				onkeydown: move |e: KeyboardEvent| {
					match e.key() {
						Key::Enter => { if let Some(raw) = editing() { commit(raw); } },
						Key::Escape => { editing.set(None); e.prevent_default(); },
						Key::ArrowUp => { nudge(step); e.prevent_default(); },
						Key::ArrowDown => { nudge(-step); e.prevent_default(); },
						_ => {},
					}
				},
			}
		}
	}
}

#[component]
fn Stat(label: String, #[props(default)] value_class: String, children: Element) -> Element {
	rsx! {
		div {
			span { class: "mb-1 block font-mono text-[0.625rem] uppercase text-main-mist/40", "{label}" }
			span { class: "text-lg font-serif font-bold {value_class}", {children} }
		}
	}
}

#[component]
fn Row(label: String, #[props(default)] value_class: String, children: Element) -> Element {
	rsx! {
		li { class: "flex justify-between",
			span { class: "text-main-mist/40", "{label}" }
			span { class: "font-bold {value_class}", {children} }
		}
	}
}

// The icons mirror the landing's lucide-react output byte-for-byte (same
// wrapper attrs, classes and path data) so the embed is a faithful port.
#[component]
fn IconPin() -> Element {
	rsx! {
		svg { xmlns: "http://www.w3.org/2000/svg", width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "lucide lucide-map-pin w-3.5 h-3.5",
			path { d: "M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0" }
			circle { cx: "12", cy: "10", r: "3" }
		}
	}
}

#[component]
fn IconArrow() -> Element {
	rsx! {
		svg { xmlns: "http://www.w3.org/2000/svg", width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "lucide lucide-arrow-up-right ml-1 w-3.5 h-3.5",
			path { d: "M7 7h10v10" }
			path { d: "M7 17 17 7" }
		}
	}
}

#[component]
fn IconTrend() -> Element {
	rsx! {
		svg { xmlns: "http://www.w3.org/2000/svg", width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "lucide lucide-trending-up w-3 h-3",
			polyline { points: "22 7 13.5 15.5 8.5 10.5 2 17" }
			polyline { points: "16 7 22 7 22 13" }
		}
	}
}
