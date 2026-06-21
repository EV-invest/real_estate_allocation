//! Iframe-embeddable marketing surface (`/embed/overview`). A standalone port of
//! the landing "Premium Asset Portfolio" bento section — no app shell, so a host
//! page can `<iframe>` it. The two property tiles deep-link into the full app
//! (`/?property=<id>`, `target=_top` so the click escapes the frame); the other
//! two tiles (market note + ROI calculator) are self-contained.

use dioxus::prelude::*;
use ev_lib::uikit::{Button, Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Skeleton, Slider};

use crate::domain::Property;

// Curated hero renders from the landing CDN — design fidelity over per-property
// pics for the marketing tile. Swap for real property images later if wanted.
const HERO_VILLA: &str = "https://d2xsxph8kpxj0f.cloudfront.net/310519663075853325/SPbgMPRFEXcrCSr7Bo27uM/luxury_villa-64wseo7dGJUQNbg7HMSNPo.webp";
const HERO_BAY: &str = "https://d2xsxph8kpxj0f.cloudfront.net/310519663075853325/SPbgMPRFEXcrCSr7Bo27uM/quynhon_future-ExoshVjhhPWYbYR4Zf3xJn.webp";

#[component]
pub fn Overview() -> Element {
	let properties = use_resource(|| async move { crate::api::list_properties(None).await.unwrap_or_default() });
	let mut tab = use_signal(|| "all".to_string());

	rsx! {
		section { id: "portfolio", class: "relative border-t border-main-mist/10 py-24",
			div { class: "mx-auto w-full max-w-[90rem] px-4",
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
					match &*properties.read() {
						None => rsx! {
							Skeleton { class: "min-h-[450px] md:col-span-2" }
							Skeleton { class: "min-h-[450px]" }
						},
						Some(list) => {
							let featured = list.first().cloned();
							let side = list.get(1).cloned();
							rsx! {
								if let Some(p) = featured {
									FeaturedCard { property: p }
								}
								if let Some(p) = side {
									SideCard { property: p }
								}
							}
						}
					}
					WhyCard {}
					Calculator {}
				}
			}
		}
	}
}

/// Large featured tile (spans two columns). The whole tile is a deep-link into
/// the full app at the property's selection URL.
#[component]
fn FeaturedCard(property: Property) -> Element {
	let href = format!("/?property={}", property.id.raw());
	rsx! {
		a {
			href,
			target: "_top",
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
					"Quy Nhơn, Bình Định"
				}
				h3 { class: "mb-4 font-serif text-2xl text-white sm:text-3xl", "{property.name}" }
				p { class: "mb-6 max-w-xl font-light text-sm text-main-mist/70 line-clamp-3", "{property.reasoning_or_terms()}" }
				div { class: "grid max-w-md grid-cols-3 gap-4 border-t border-main-mist/10 pt-6",
					match property.price {
						Some(p) => rsx! { Stat { label: "Price", value_class: "text-main-accent-t3", "{p}" } },
						None => rsx! { Stat { label: "Price", value_class: "text-warn", "?" } },
					}
					Stat { label: "Status", value_class: "text-main-accent-t2", "Purchased" }
					Stat { label: "Action", value_class: "text-white", "Open ↗" }
				}
			}
		}
	}
}

/// Standard side tile. Also a deep-link into the full app.
#[component]
fn SideCard(property: Property) -> Element {
	let href = format!("/?property={}", property.id.raw());
	rsx! {
		a {
			href,
			target: "_top",
			class: "group relative flex min-h-[450px] flex-col justify-end overflow-hidden border border-main-mist/10 bg-main-black/40",
			div {
				class: "absolute inset-0 z-0 bg-cover bg-center transition-transform duration-700 group-hover:scale-105",
				style: "background-image: linear-gradient(to top, rgba(7,13,24,0.96) 20%, rgba(7,13,24,0.4)), url({HERO_BAY})",
			}
			div { class: "relative z-10 p-8",
				div { class: "mb-3 flex items-center gap-2 font-mono text-xs text-main-accent-t1",
					IconPin {}
					"Quy Nhơn, Bình Định"
				}
				h3 { class: "mb-4 font-serif text-xl text-white sm:text-2xl", "{property.name}" }
				p { class: "mb-6 font-light text-sm text-main-mist/70 line-clamp-3", "{property.reasoning_or_terms()}" }
				div { class: "flex items-center justify-between border-t border-main-mist/10 pt-6",
					match property.price {
						Some(p) => rsx! { Stat { label: "Price", value_class: "text-white", "{p}" } },
						None => rsx! { Stat { label: "Price", value_class: "text-warn", "?" } },
					}
					span { class: "inline-flex items-center gap-1 font-mono text-xs tracking-wider text-main-accent-t1 group-hover:text-white",
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

/// Client-side ROI projector (spans two columns). Mirrors the landing model:
/// per-class annual yield + appreciation, compounded over the term.
#[component]
fn Calculator() -> Element {
	let mut amount = use_signal(|| 100_000.0_f64);
	let mut term = use_signal(|| 5_u32);
	let mut commercial = use_signal(|| false);

	let (rate, appr): (f64, f64) = if commercial() { (0.12, 0.18) } else { (0.085, 0.15) };
	let total = amount() * (1.0 + rate + appr).powi(term() as i32);
	let profit = total - amount();
	let roi = profit / amount() * 100.0;

	rsx! {
		div { class: "grid grid-cols-1 gap-8 border border-main-mist/10 bg-main-card p-8 md:col-span-2 md:grid-cols-2",
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
						label { class: "mb-3 block uppercase text-muted-foreground", "Principal Investment ($ USD)" }
						Slider {
							class: "[&_[data-slot=slider-track]]:bg-main-black/50 [&_[data-slot=slider-range]]:bg-main-accent-t1 [&_[data-slot=slider-thumb]]:border-main-accent-t1",
							min: 50_000.0,
							max: 1_000_000.0,
							step: 10_000.0,
							value: amount(),
							on_value_change: move |v| amount.set(v),
						}
						div { class: "mt-2 flex justify-between font-bold text-main-accent-t1",
							span { "$50k" }
							span { class: "text-sm", "{usd(amount())}" }
							span { "$1M" }
						}
					}
					div { class: "grid grid-cols-2 gap-4",
						div {
							label { class: "mb-2 block uppercase text-muted-foreground", "Term (Years)" }
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
							label { class: "mb-2 block uppercase text-muted-foreground", "Asset Type" }
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
						span { class: "mb-1 block font-mono text-[10px] uppercase text-muted-foreground", "Estimated ROI" }
						span { class: "font-serif text-4xl font-bold text-main-accent-t3", "{roi:.1}%" }
					}
					div { class: "grid grid-cols-2 gap-4 border-t border-main-mist/10 pt-4",
						div {
							span { class: "mb-0.5 block font-mono text-[9px] uppercase text-muted-foreground", "Total Payout" }
							span { class: "font-mono text-sm font-bold text-white", "{usd(total)}" }
						}
						div {
							span { class: "mb-0.5 block font-mono text-[9px] uppercase text-muted-foreground", "Net Profit" }
							span { class: "font-mono text-sm font-bold text-main-accent-t2", "{usd(profit)}" }
						}
					}
				}
				div { class: "mt-6",
					p { class: "mb-4 text-[10px] font-light leading-relaxed text-muted-foreground",
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
			span { class: "mb-1 block font-mono text-[10px] uppercase text-muted-foreground", "{label}" }
			span { class: "text-lg font-serif font-bold {value_class}", {children} }
		}
	}
}

#[component]
fn Row(label: String, #[props(default)] value_class: String, children: Element) -> Element {
	rsx! {
		li { class: "flex justify-between",
			span { class: "text-muted-foreground", "{label}" }
			span { class: "font-bold {value_class}", {children} }
		}
	}
}

impl Property {
	/// Short blurb for the marketing tiles: prefer the (richer) reasoning, fall
	/// back to terms. `line-clamp` keeps it to a few lines in the card.
	fn reasoning_or_terms(&self) -> &str {
		self.additional_reasoning.as_deref().or(self.terms.as_deref()).unwrap_or_default()
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
