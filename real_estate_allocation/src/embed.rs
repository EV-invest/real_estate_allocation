//! Iframe-embeddable marketing surface (`/embed/overview`). A standalone port of
//! the landing "Premium Asset Portfolio" bento section — no app shell, so a host
//! page can `<iframe>` it. Static content mirroring the landing source; the only
//! interactive tile is the self-contained ROI calculator.

use dioxus::prelude::*;
use ev_lib::uikit::{Button, ButtonVariant, Container, Select, SelectContent, SelectItem, SelectTrigger, SelectValue};

// Hero renders from the landing CDN — the same `ASSETS.luxury_villa` /
// `ASSETS.quynhon_future` the original section references.
const HERO_VILLA: &str = "https://d2xsxph8kpxj0f.cloudfront.net/310519663075853325/SPbgMPRFEXcrCSr7Bo27uM/luxury_villa-64wseo7dGJUQNbg7HMSNPo.webp";
const HERO_BAY: &str = "https://d2xsxph8kpxj0f.cloudfront.net/310519663075853325/SPbgMPRFEXcrCSr7Bo27uM/quynhon_future-ExoshVjhhPWYbYR4Zf3xJn.webp";

const A_MIN: f64 = 50_000.0;
const A_MAX: f64 = 1_000_000.0;
const A_STEP: f64 = 10_000.0;
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

/// Large featured tile (spans two columns). Static marketing content, mirroring
/// the landing source byte-for-byte.
#[component]
fn FeaturedCard() -> Element {
	rsx! {
		div {
			class: "group relative flex min-h-[450px] flex-col justify-end overflow-hidden border border-main-mist/10 bg-main-black/40 md:col-span-2",
			div {
				class: "absolute inset-0 z-0 bg-cover bg-center transition-transform duration-700 group-hover:scale-105",
				style: "background-image: linear-gradient(to top, rgba(7,13,24,0.96) 10%, rgba(7,13,24,0.2)), url({HERO_VILLA})",
			}
			div { class: "absolute right-4 top-4 bg-main-accent-t1 px-3 py-1.5 font-mono text-[10px] font-bold uppercase tracking-widest text-main-black",
				"Featured Deal"
			}
			div { class: "relative z-10 p-8",
				div { class: "mb-3 flex items-center gap-2 font-mono text-xs text-main-accent-t1",
					IconPin {}
					"Nhon Ly Beach, Quy Nhon"
				}
				h3 { class: "mb-4 font-serif text-2xl text-white sm:text-3xl", "The Horizon Premium Villas" }
				p { class: "mb-6 max-w-xl font-light text-sm text-main-mist/70",
					"Exclusive ultra-luxury oceanfront villas with private pools, nestled between pristine limestone cliffs and crystal-clear turquoise waters."
				}
				div { class: "grid max-w-md grid-cols-3 gap-4 border-t border-main-mist/10 pt-6",
					Stat { label: "Target Yield", value_class: "text-main-accent-t2", "12.5% p.a." }
					Stat { label: "Appreciation", value_class: "text-main-accent-t3", "18% YoY" }
					Stat { label: "Status", value_class: "text-white", "Pre-Launch" }
				}
			}
		}
	}
}

/// Standard side tile. Static marketing content.
#[component]
fn SideCard() -> Element {
	rsx! {
		div {
			class: "group relative flex min-h-[450px] flex-col justify-end overflow-hidden border border-main-mist/10 bg-main-black/40",
			div {
				class: "absolute inset-0 z-0 bg-cover bg-center transition-transform duration-700 group-hover:scale-105",
				style: "background-image: linear-gradient(to top, rgba(7,13,24,0.96) 20%, rgba(7,13,24,0.4)), url({HERO_BAY})",
			}
			div { class: "relative z-10 p-8",
				div { class: "mb-3 flex items-center gap-2 font-mono text-xs text-main-accent-t1",
					IconPin {}
					"Quy Nhon Center"
				}
				h3 { class: "mb-4 font-serif text-xl text-white sm:text-2xl", "Quy Nhon Bay Residences" }
				p { class: "mb-6 font-light text-sm text-main-mist/70",
					"Premium high-rise apartments with panoramic views of the bay, integrating luxury amenities and smart-home technology."
				}
				div { class: "flex items-center justify-between border-t border-main-mist/10 pt-6",
					div {
						span { class: "mb-0.5 block font-mono text-[9px] uppercase text-main-mist/40", "LTV Ratio" }
						span { class: "text-sm font-serif font-bold text-white", "55% Max" }
					}
					Button {
						variant: ButtonVariant::Ghost,
						class: "p-0 font-mono text-xs tracking-wider text-main-accent-t1 hover:bg-transparent hover:text-white",
						"Deal Sheet"
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
				div { class: "mb-6 inline-flex items-center gap-1.5 border border-main-accent-t1/20 bg-main-accent-t1/10 px-2 py-1 font-mono text-[9px] uppercase tracking-wider text-main-accent-t1",
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

// Principal slider bounds, in USD. The slider below is hand-inlined from the uikit
// `Slider`'s compiled markup (colours applied directly, not via arbitrary-variant
// overrides), so the track fill and round thumb don't depend on `cn!` merge survival.

fn snap(v: f64) -> f64 {
	let v = v.clamp(A_MIN, A_MAX);
	(((v - A_MIN) / A_STEP).round() * A_STEP + A_MIN).clamp(A_MIN, A_MAX)
}

/// Client-side ROI projector (spans two columns). Mirrors the landing model:
/// per-class annual yield + appreciation, compounded over the term.
#[component]
fn Calculator() -> Element {
	let mut amount = use_signal(|| 100_000.0_f64);
	let mut term = use_signal(|| 5_u32);
	let mut commercial = use_signal(|| false);

	// Slider drag state: the track's measured origin/width on the x-axis.
	let mut track = use_signal(|| Option::<std::rc::Rc<MountedData>>::None);
	let mut bounds = use_signal(|| (0.0_f64, 1.0_f64));
	let mut dragging = use_signal(|| false);
	let pct = ((amount() - A_MIN) / (A_MAX - A_MIN) * 100.0).clamp(0.0, 100.0);

	let (rate, appr): (f64, f64) = if commercial() { (0.12, 0.18) } else { (0.085, 0.15) };
	let total = amount() * (1.0 + rate + appr).powi(term() as i32);
	let profit = total - amount();
	let roi = profit / amount() * 100.0;

	rsx! {
		div { id: "calculator", class: "grid grid-cols-1 gap-8 border border-main-mist/10 bg-main-card p-8 md:col-span-2 md:grid-cols-2",
			div { class: "flex flex-col justify-between",
				div {
					span { class: "mb-3 block font-mono text-xs uppercase tracking-widest text-main-accent-t1",
						"Yield Terminal"
					}
					h3 { class: "mb-4 font-serif text-2xl text-white sm:text-3xl", "Investment Calculator" }
					p { class: "mb-6 font-light text-sm text-main-mist/70",
						"Project your returns across different asset classes in Quy Nhon based on our current fund advisory models."
					}
				}
				div { class: "space-y-4 font-mono text-xs",
					div {
						label { class: "mb-3 block uppercase text-main-mist/40", "Principal Investment ($ USD)" }
						span {
		//TODO!!!: replace with kit's Slider
							class: "relative flex w-full touch-none select-none items-center",
							onpointerdown: move |e: PointerEvent| async move {
								let Some(t) = track() else { return };
								let Ok(rect) = t.get_client_rect().await else { return };
								bounds.set((rect.origin.x, rect.size.width));
								dragging.set(true);
								let ratio = (e.client_coordinates().x - rect.origin.x) / rect.size.width.max(f64::EPSILON);
								amount.set(snap(A_MIN + ratio * (A_MAX - A_MIN)));
							},
							onpointermove: move |e: PointerEvent| {
								if !dragging() {
									return;
								}
								let (ox, w) = bounds();
								let ratio = (e.client_coordinates().x - ox) / w.max(f64::EPSILON);
								amount.set(snap(A_MIN + ratio * (A_MAX - A_MIN)));
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
								class: "block size-4 shrink-0 rounded-full border border-main-accent-t1 bg-white shadow-sm",
								style: "position: absolute; left: {pct}%; top: 50%; transform: translate(-50%, -50%);",
								role: "slider",
								tabindex: "0",
								"aria-valuenow": amount(),
								"aria-valuemin": A_MIN,
								"aria-valuemax": A_MAX,
								onkeydown: move |e: KeyboardEvent| {
									let next = match e.key() {
										Key::ArrowRight | Key::ArrowUp => amount() + A_STEP,
										Key::ArrowLeft | Key::ArrowDown => amount() - A_STEP,
										Key::Home => A_MIN,
										Key::End => A_MAX,
										_ => return,
									};
									e.prevent_default();
									amount.set(snap(next));
								},
							}
						}
						div { class: "mt-2 flex justify-between font-bold text-main-accent-t1",
							span { "$50k" }
							span { class: "text-sm", "{usd(amount())}" }
							span { "$1M" }
						}
					}
					div { class: "grid grid-cols-2 gap-4",
						div {
							label { class: "mb-2 block uppercase text-main-mist/40", "Term (Years)" }
							Select {
								value: term().to_string(),
								on_value_change: move |v: String| {
									if let Ok(y) = v.parse() { term.set(y); }
								},
								SelectTrigger { class: "w-full border-main-mist/20 bg-main-black/60 font-mono", SelectValue {} }
								SelectContent {
									for y in [3u32, 5, 7, 10] {
										SelectItem { value: "{y}", "{y} Years" }
									}
								}
							}
						}
						div {
							label { class: "mb-2 block uppercase text-main-mist/40", "Asset Type" }
							Select {
								value: if commercial() { "commercial".to_string() } else { "residential".to_string() },
								on_value_change: move |v: String| commercial.set(v == "commercial"),
								SelectTrigger { class: "w-full border-main-mist/20 bg-main-black/60 font-mono", SelectValue {} }
								SelectContent {
									SelectItem { value: "residential", "Luxury Villa" }
									SelectItem { value: "commercial", "Commercial Hub" }
								}
							}
						}
					}
				}
			}

			// Output panel
			div { class: "flex flex-col justify-between border border-main-mist/10 bg-main-black/40 p-6",
				div { class: "space-y-4",
					div {
						span { class: "mb-1 block font-mono text-[10px] uppercase text-main-mist/40", "Estimated ROI" }
						span { class: "font-serif text-4xl font-bold text-main-accent-t3", "{roi:.1}%" }
					}
					div { class: "grid grid-cols-2 gap-4 border-t border-main-mist/10 pt-4",
						div {
							span { class: "mb-0.5 block font-mono text-[9px] uppercase text-main-mist/40", "Total Payout" }
							span { class: "font-mono text-sm font-bold text-white", "{usd(total)}" }
						}
						div {
							span { class: "mb-0.5 block font-mono text-[9px] uppercase text-main-mist/40", "Net Profit" }
							span { class: "font-mono text-sm font-bold text-main-accent-t2", "{usd(profit)}" }
						}
					}
				}
				div { class: "mt-6",
					p { class: "mb-4 text-[10px] font-light leading-relaxed text-main-mist/40",
						"*Projections are based on historical performance and regional growth targets. Actual results may vary."
					}
					Button { class: "w-full rounded-none bg-main-accent-t1 py-5 font-mono text-xs uppercase tracking-wider text-main-black hover:bg-main-mist hover:text-main-brand",
						"Request advisory"
					}
				}
			}
		}
	}
}

#[component]
fn Stat(label: String, #[props(default)] value_class: String, children: Element) -> Element {
	rsx! {
		div {
			span { class: "mb-1 block font-mono text-[10px] uppercase text-main-mist/40", "{label}" }
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

/// Whole-dollar USD with thousands separators: `$338,200`.
fn usd(n: f64) -> String {
	let v = n.round() as i64;
	let digits = v.abs().to_string();
	let bytes = digits.as_bytes();
	let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
	for (i, b) in bytes.iter().enumerate() {
		if i > 0 && (bytes.len() - i) % 3 == 0 {
			out.push(',');
		}
		out.push(*b as char);
	}
	format!("${out}")
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
