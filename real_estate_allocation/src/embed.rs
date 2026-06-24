//! Marketing surface mounted by the cross-origin microfrontend bundle
//! (`real_estate_allocation_mfe`). A standalone port of the landing "Premium Asset
//! Portfolio" bento section — no app shell, so the landing host composes `<tag>`
//! directly into its page. Static content mirroring the landing source; the only
//! interactive tile is the self-contained correlation / risk-premia terminal.

use dioxus::prelude::*;
use ev_lib::{mfe::bundle_origin, uikit::Container};

use crate::factors::profile;

// Both tiles are real listings. Banners are served from the bundle's own origin
// under `/mfe/seed/…` (copied there by the build) — paths relative to that origin,
// resolved against `bundle_origin()` at render. A click breaks out to the REA
// dashboard home with the property pre-selected (absolute, cross-origin link).
const Q1_BANNER: &str = "seed/q1_tower/render.jpg";
const Q1_PROPERTY: &str = "b41510ef-1e74-4d4f-a15c-1dfafdd0ee5a";
const TMS_BANNER: &str = "seed/tms/building.jpg";
const TMS_PROPERTY: &str = "c19bded1-1a13-49ad-a0f0-549b2aec2d0e";

// Swap-fraction slider bounds, in percent (0–100% of the host book moved into S).
const A_MIN: f64 = 0.0;
const A_MAX: f64 = 100.0;
const A_STEP: f64 = 1.0;
#[component]
pub fn Overview() -> Element {
	let mut tab = use_signal(|| "all".to_string());

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

				// Filter tabs — visual parity with the source; cosmetic only.
				div { class: "mb-12 flex flex-wrap gap-2 border-b border-main-mist/10 pb-4 font-mono text-xs tracking-wider",
					for t in ["all", "villas", "commercial", "land"] {
						button {
							key: "{t}",
							class: if tab() == t { "bg-main-accent-t1 px-5 py-2.5 font-bold uppercase text-main-black transition-all duration-300" } else { "px-5 py-2.5 uppercase text-main-mist/60 transition-all duration-300 hover:bg-main-mist/5 hover:text-white" },
							onclick: move |_| tab.set(t.to_string()),
							"{t}"
						}
					}
				}

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

/// Large featured tile (spans two columns). Links to the Q1 Tower property page.
#[component]
fn FeaturedCard() -> Element {
	// Pull Q1's live figures so the headline stats track the DB rather than hard-coded
	// copy. `get_building` populates `price_series`, the basis for `appreciation_yoy`.
	let building = use_resource(|| async move {
		let id = crate::domain::parse_building_id(Q1_PROPERTY).ok()?;
		crate::api::get_building(id).await.ok().flatten()
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

	rsx! {
		a {
			href: "{origin}/?building={Q1_PROPERTY}",
			class: "group relative flex min-h-[450px] flex-col justify-end overflow-hidden border border-main-mist/10 bg-main-black/40 md:col-span-2",
			div {
				class: "absolute inset-0 z-0 bg-cover bg-center transition-transform duration-700 group-hover:scale-105",
				style: "background-image: linear-gradient(to top, rgba(7,13,24,0.96) 10%, rgba(7,13,24,0.2)), url({origin}/mfe/{Q1_BANNER})",
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
	rsx! {
		a {
			href: "{origin}/?building={TMS_PROPERTY}",
			class: "group relative flex min-h-[450px] flex-col justify-end overflow-hidden border border-main-mist/10 bg-main-black/40",
			div {
				class: "absolute inset-0 z-0 bg-cover bg-center transition-transform duration-700 group-hover:scale-105",
				style: "background-image: linear-gradient(to top, rgba(7,13,24,0.96) 20%, rgba(7,13,24,0.4)), url({origin}/mfe/{TMS_BANNER})",
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

/// Correlation / risk-premia terminal (spans two columns). Shows our instrument's
/// correlation profile vs the popular alpha factors and, under probabilistic-Kelly
/// sizing (γ=1), what swapping `w%` of a host book into us does to its effective risk
/// premia (risk cost = σ²/2) and compound performance. See `crate::factors`.
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

	rsx! {
		div { id: "calculator", class: "grid grid-cols-1 gap-3 border border-main-mist/10 bg-main-card p-4 md:col-span-2 md:grid-cols-2",
			// Heading + swap slider
			div { class: "flex flex-col",
				div { class: "flex items-center gap-2",
					span { class: "font-mono text-xs uppercase tracking-widest text-main-accent-t1", "Risk Terminal" }
					h3 { class: "font-serif text-lg text-white", "Correlation Profile" }
				}
				p { class: "mb-2 font-light text-xs text-main-mist/70",
					"We are judged on our marginal effect on your book — accretive because we are nearly uncorrelated with the alpha factors you already own."
				}
				div { class: "mt-auto font-mono text-[0.6875rem]",
					label { class: "mb-1.5 block uppercase text-main-mist/40", "Allocation swapped into Vietnam (%)" }
					span {
						class: "relative flex w-full touch-none select-none items-center",
						onpointerdown: move |e: PointerEvent| async move {
							let Some(t) = track() else { return };
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
						onpointerleave: move |_| dragging.set(false),
						span {
							class: "relative h-1.5 w-full grow overflow-hidden rounded-full bg-main-black/50",
							onmounted: move |e: MountedEvent| track.set(Some(e.data())),
							span {
								class: "absolute h-full bg-main-accent-t1",
								style: "width: {pct}%;",
							}
						}
						span {
							class: "block size-3 shrink-0 rounded-full border border-main-accent-t1 bg-white shadow-sm",
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
					div { class: "mt-1 flex justify-between font-bold text-main-accent-t1",
						span { "0%" }
						span { "{swap():.0}%" }
						span { "100%" }
					}
				}
			}

			// Output panel
			div { class: "flex flex-col justify-between border border-main-mist/10 bg-main-black/40 p-3",
				div {
					span { class: "mb-0.5 block font-mono text-[0.5rem] uppercase text-main-mist/40", "Δ Effective Risk Premia" }
					span { class: "font-serif text-2xl font-bold text-main-accent-t3", "{out.delta_risk_premia * 10_000.0:+.1} bps" }
					div { class: "mt-2 flex gap-4 border-t border-main-mist/10 pt-2",
						div {
							span { class: "block font-mono text-[0.4375rem] uppercase text-main-mist/40", "Δ Expected Perf" }
							span { class: "font-mono text-[0.6875rem] font-bold text-main-accent-t2", "{out.delta_performance * 100.0:+.2}%" }
						}
						div {
							span { class: "block font-mono text-[0.4375rem] uppercase text-main-mist/40", "ρ (S, P)" }
							span { class: "font-mono text-[0.6875rem] font-bold text-white", "{out.rho_sp:+.2}" }
						}
					}
				}
				p { class: "mt-2 text-[0.4375rem] font-light leading-tight text-main-mist/30",
					"*Correlation figures indicative placeholders. Risk cost under probabilistic-Kelly (γ≈1). Actual results may vary."
				}
			}

			// Factor exposure grid — spans both columns.
			div { class: "font-mono text-[0.6875rem] md:col-span-2",
				div { class: "grid grid-cols-2 gap-1 sm:grid-cols-3 md:grid-cols-4",
					for (f, w) in p.factors.iter().zip(exposures.iter().copied()) {
						StepperCell { label: f.label.to_string(), value: w, step: 1.0, min: 0.0, max: 100.0, suffix: "%", rho: f.rho }
					}
					StepperCell { label: "Host YoY return".to_string(), value: yoy, step: 0.5, min: -50.0, max: 100.0, suffix: "%" }
				}
			}
		}
	}
}

/// TradingView-style numeric cell: label left, bordered value box right with hover-only
/// up/down chevrons, vertical pointer-drag to scrub, and ↑/↓ keyboard nudge. Reused for
/// every factor exposure and the host YoY input. An optional `rho` renders our read-only
/// correlation to that factor beside the box (the "profile vs factors" picture).
#[component]
fn StepperCell(label: String, value: Signal<f64>, step: f64, min: f64, max: f64, #[props(default)] suffix: &'static str, #[props(default)] rho: Option<f64>) -> Element {
	let mut value = value;
	let mut text = use_signal(|| format!("{:.1}", value()));
	let mut drag = use_signal(|| Option::<(f64, f64)>::None);

	rsx! {
		div { class: "group flex items-center justify-between gap-1 rounded border border-main-mist/20 bg-main-black/60 px-1.5 py-0 font-mono text-[0.625rem]",
			span { class: "truncate uppercase text-main-mist/50", "{label}" }
			div { class: "flex items-center gap-1",
				if let Some(r) = rho {
					span {
						class: if r >= 0.0 { "text-[0.5625rem] text-main-accent-t2" } else { "text-[0.5625rem] text-main-accent-t3" },
						"{r:+.2}"
					}
				}
				div {
					class: "group/inner flex touch-none select-none items-center rounded border border-main-mist/20 bg-main-black/40",
					onpointerdown: move |e: PointerEvent| drag.set(Some((e.client_coordinates().y, value()))),
					onpointermove: move |e: PointerEvent| {
						let Some((y0, v0)) = drag() else { return };
						let n = v0 + ((y0 - e.client_coordinates().y) / 8.0).round() * step;
						value.set(n.clamp(min, max));
						text.set(format!("{value:.1}"));
					},
					onpointerup: move |_| drag.set(None),
					onpointerleave: move |_| drag.set(None),
					input {
						r#type: "text",
						inputmode: "decimal",
						class: "w-[4ch] bg-transparent px-0.5 py-0 text-right text-white outline-none",
						value: "{text}{suffix}",
						oninput: move |e: FormEvent| text.set(e.value()),
						onblur: move |_| {
							if let Ok(v) = text().parse::<f64>() {
								value.set(v.clamp(min, max));
								text.set(format!("{value:.1}"));
							} else {
								text.set(format!("{value:.1}"));
							}
						},
						onkeydown: move |e: KeyboardEvent| {
							match e.key() {
								Key::Enter => {
									if let Ok(v) = text().parse::<f64>() {
										value.set(v.clamp(min, max));
										text.set(format!("{value:.1}"));
									} else {
										text.set(format!("{value:.1}"));
									}
								},
								Key::ArrowUp => { value.set((value() + step).clamp(min, max)); text.set(format!("{value:.1}")); e.prevent_default(); },
								Key::ArrowDown => { value.set((value() - step).clamp(min, max)); text.set(format!("{value:.1}")); e.prevent_default(); },
								Key::Escape => { text.set(format!("{value:.1}")); e.prevent_default(); },
								_ => {},
							}
						},
					}
					div { class: "flex flex-col opacity-0 transition-opacity group-hover/inner:opacity-100",
						button {
							r#type: "button",
							class: "leading-none text-main-mist/50 hover:text-white",
							onclick: move |_| { value.set((value() + step).clamp(min, max)); text.set(format!("{value:.1}")); },
							IconChevron { up: true }
						}
						button {
							r#type: "button",
							class: "leading-none text-main-mist/50 hover:text-white",
							onclick: move |_| { value.set((value() - step).clamp(min, max)); text.set(format!("{value:.1}")); },
							IconChevron { up: false }
						}
					}
				}
			}
		}
	}
}

#[component]
fn IconChevron(up: bool) -> Element {
	rsx! {
		svg { xmlns: "http://www.w3.org/2000/svg", width: "24", height: "24", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", class: "h-3 w-3",
			if up {
				path { d: "m18 15-6-6-6 6" }
			} else {
				path { d: "m6 9 6 6 6-6" }
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
