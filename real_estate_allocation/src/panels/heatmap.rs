use dioxus::prelude::*;
use ev_lib::uikit::{Card, CardContent, Skeleton};

use crate::{
	app::Selected,
	domain::{Property, PropertyId, PropertyStateKind},
};

/// Portfolio heatmap: a treemap of every holding, area ∝ price, colour ∝ recent
/// value change, with the selected property highlighted and each tile clickable.
#[component]
pub fn PortfolioHeatmap() -> Element {
	// Shares the map's `?selection=` filter, minus `Interesting`: the heatmap is the
	// committed book, so prospects never enter the treemap. Owned tiles are solid;
	// purchasing ones are striped over the heat fill.
	let filter = use_context::<crate::app::Filter>();
	let properties = use_resource(move || {
		let states = query_states(&filter());
		async move { crate::api::list_properties(Some(states)).await.unwrap_or_default() }
	});

	rsx! {
		// h-full so the treemap fills (and reflows with) its dock pane; the tab already labels it,
		// so the redundant card header is gone — only the loss/gain legend survives, overlaid.
		Card { class: "flex h-full flex-col",
			CardContent { class: "relative flex-1",
				match &*properties.read() {
					None => rsx! { Skeleton { class: "h-full w-full" } },
					Some(list) if list.is_empty() => rsx! {
						div { class: "flex h-full items-center justify-center text-sm text-muted-foreground",
							"No holdings to display."
						}
					},
					Some(list) => rsx! { Treemap { properties: list.clone() } },
				}
			}
		}
	}
}

/// The heatmap's committed-book filter: the chip states minus `Interesting` (a
/// prospect, never drawn). Empty in → empty out → empty treemap. The store query
/// is the OR over exactly these states, so an empty set draws nothing.
fn query_states(filter: &[PropertyStateKind]) -> Vec<PropertyStateKind> {
	filter.iter().copied().filter(|s| *s != PropertyStateKind::Interesting).collect()
}

/// One fully-resolved tile: geometry + everything needed to paint it. Built once
/// from the property list so the view is a single pass over `Vec<Tile>` — no
/// parallel `rects[i]` indexing, no per-tile recomputation mid-render.
#[derive(Clone, PartialEq)]
struct Tile {
	id: PropertyId,
	name: String,
	rect: Rect,
	change: f64,
	/// Not-yet-owned (under-construction / interesting): drawn provisional —
	/// dimmed + hatched over the heat fill.
	prospect: bool,
}

impl Tile {
	fn layout(properties: &[Property]) -> Vec<Self> {
		// Unknown-price holdings (e.g. still `Purchasing`) get the *mean* known price,
		// not an absolute `1.0`: beside six-figure holdings a flat 1.0 collapses to a
		// zero-area sliver and the tile vanishes. Mean keeps it visibly present without
		// inventing a figure. No known prices at all → equal weights.
		let known: Vec<f64> = properties.iter().filter_map(|p| p.price.map(|m| m.amount())).filter(|a| *a > 0.0).collect();
		let fallback = if known.is_empty() { 1.0 } else { known.iter().sum::<f64>() / known.len() as f64 };
		let values: Vec<f64> = properties.iter().map(|p| p.price.map_or(fallback, |m| m.amount()).max(1.0)).collect();
		let rects = squarify(&values, Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 });
		properties
			.iter()
			.zip(rects)
			.map(|(p, rect)| Tile {
				id: p.id,
				name: p.name.clone(),
				rect,
				change: mock_change(p.id),
				prospect: p.state.kind() != PropertyStateKind::Purchased,
			})
			.collect()
	}
}

#[component]
fn Treemap(properties: Vec<Property>) -> Element {
	let mut selected = use_context::<Selected>();
	let tiles = Tile::layout(&properties);

	rsx! {
		div { class: "relative h-full w-full overflow-hidden rounded-lg bg-main-surface",
			for t in tiles {
				{
					let Tile { id, name, rect: r, change, prospect } = t;
					let is_sel = selected() == Some(id);
					let ring = if is_sel { "ring-2 ring-main-accent-t1 z-10" } else { "" };
					let (dim, stripes) = crate::uikit::provisional(prospect);
					rsx! {
						button {
							key: "{id}",
							class: "absolute flex flex-col justify-between overflow-hidden rounded-md p-2.5 text-left transition-[filter] hover:brightness-110 {ring} {dim}",
							style: "left:calc({r.x:.4}% + 2px);top:calc({r.y:.4}% + 2px);width:calc({r.w:.4}% - 4px);height:calc({r.h:.4}% - 4px);background-color:{heat_color(change)}{stripes}",
							onclick: move |_| selected.set(Some(id)),
							div { class: "flex min-w-0 flex-col gap-0.5",
								span { class: "truncate text-sm font-semibold text-white", "{name}" }
								span { class: "text-xs text-white/80", "{change:+.1}%" }
							}
							if is_sel {
								span { class: "text-[10px] font-semibold text-white/90", "● This property" }
							}
						}
					}
				}
			}
		}
	}
}

/// No real performance data lives in the domain — `price_series` is itself a mock
/// filled only by `get_property` — so derive a STABLE pseudo-change from the id.
/// Same property always paints the same colour; range ≈ [-4%, +6%].
fn mock_change(id: PropertyId) -> f64 {
	let seed = id.raw().as_u64_pair().0;
	((seed % 1000) as f64 / 1000.0) * 10.0 - 4.0
}

fn heat_color(change: f64) -> String {
	if change >= 0.0 {
		let t = (change / 6.0).clamp(0.0, 1.0);
		lerp_rgb((0x1f, 0x4d, 0x33), (0x1f, 0x8a, 0x4c), t)
	} else {
		let t = (-change / 4.0).clamp(0.0, 1.0);
		lerp_rgb((0x4a, 0x2b, 0x2c), (0x9e, 0x2f, 0x2c), t)
	}
}

fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> String {
	let c = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
	format!("rgb({},{},{})", c(a.0, b.0), c(a.1, b.1), c(a.2, b.2))
}

#[derive(Clone, Copy, PartialEq)]
struct Rect {
	x: f64,
	y: f64,
	w: f64,
	h: f64,
}

/// Largest aspect ratio in a row, given the side it's laid along. Lower is squarer.
fn worst(row: &[f64], side: f64) -> f64 {
	let sum: f64 = row.iter().sum();
	if sum <= 0.0 {
		return f64::INFINITY;
	}
	let max = row.iter().copied().fold(0.0_f64, f64::max);
	let min = row.iter().copied().fold(f64::INFINITY, f64::min);
	let s2 = sum * sum;
	let w2 = side * side;
	(w2 * max / s2).max(s2 / (w2 * min))
}

/// Place a finished row along the shorter side of `free`, then shrink `free`.
fn layout_row(row: &[(usize, f64)], free: &mut Rect, out: &mut [Rect]) {
	let sum: f64 = row.iter().map(|&(_, a)| a).sum();
	if free.w >= free.h {
		let w = sum / free.h;
		let mut y = free.y;
		for &(i, a) in row {
			let h = a / w;
			out[i] = Rect { x: free.x, y, w, h };
			y += h;
		}
		free.x += w;
		free.w -= w;
	} else {
		let h = sum / free.w;
		let mut x = free.x;
		for &(i, a) in row {
			let w = a / h;
			out[i] = Rect { x, y: free.y, w, h };
			x += w;
		}
		free.y += h;
		free.h -= h;
	}
}

/// Squarified treemap (Bruls–Huizing–van Wijk): partition `bounds` into one rect
/// per value, area ∝ value, keeping rects close to square. Output is in input
/// order; zero/empty input yields zero-area rects.
fn squarify(values: &[f64], bounds: Rect) -> Vec<Rect> {
	let mut out = vec![
		Rect {
			x: bounds.x,
			y: bounds.y,
			w: 0.0,
			h: 0.0
		};
		values.len()
	];
	let total: f64 = values.iter().sum();
	if total <= 0.0 {
		return out;
	}
	let scale = (bounds.w * bounds.h) / total;
	let mut order: Vec<usize> = (0..values.len()).collect();
	order.sort_by(|&a, &b| values[b].partial_cmp(&values[a]).unwrap_or(std::cmp::Ordering::Equal));

	let mut free = bounds;
	let mut row: Vec<(usize, f64)> = Vec::new();
	let mut areas: Vec<f64> = Vec::new();
	for &idx in &order {
		let area = values[idx] * scale;
		let side = free.w.min(free.h);
		let mut trial = areas.clone();
		trial.push(area);
		if row.is_empty() || worst(&trial, side) <= worst(&areas, side) {
			row.push((idx, area));
			areas.push(area);
		} else {
			layout_row(&row, &mut free, &mut out);
			row.clear();
			areas.clear();
			row.push((idx, area));
			areas.push(area);
		}
	}
	if !row.is_empty() {
		layout_row(&row, &mut free, &mut out);
	}
	out
}

#[cfg(test)]
mod tests {
	use jiff::Timestamp;
	use uuid::Uuid;

	use super::*;
	use crate::domain::{ConstructionStatus, GooglePlace, Money, PropertyState, ResearchUrl};

	// `mock_change` keys off the id's high 64 bits → seed those for stable heat colours.
	fn prop(seed: u64, name: &str, price: Option<f64>, state: PropertyState) -> Property {
		Property {
			id: PropertyId::from_raw(Uuid::from_u128((seed as u128) << 64)),
			name: name.into(),
			place: GooglePlace::parse("place".into()).unwrap(),
			price: price.map(|p| Money::parse(p).unwrap()),
			state,
			construction: ConstructionStatus::Completed,
			target_appreciation: 0.0,
			developer: None,
			research_url: ResearchUrl::parse("https://x.test".into()).unwrap(),
			terms: None,
			deal: None,
			loan: None,
			additional_reasoning: None,
			price_series: vec![],
			coords: None,
		}
	}

	/// Mirrors the real book: priced `Purchased` holdings plus price-less `Purchasing`
	/// ones — the exact mix that made the purchasing tiles vanish.
	fn mixed_book() -> Vec<Property> {
		let bought: Timestamp = "2024-01-01T00:00:00Z".parse().unwrap();
		vec![
			prop(250, "Quy Nhon Melody", Some(96000.0), PropertyState::Purchased(bought)),
			prop(700, "Vina2 Panorama", Some(60000.0), PropertyState::Purchased(bought)),
			prop(500, "Ecolife Riverside", Some(59000.0), PropertyState::Purchased(bought)),
			prop(150, "The Calla", Some(80000.0), PropertyState::Purchased(bought)),
			prop(900, "Q1 Tower", None, PropertyState::Purchasing),
			prop(350, "Triton", None, PropertyState::Purchasing),
		]
	}

	fn render(properties: Vec<Property>) -> String {
		#[component]
		fn Harness(properties: Vec<Property>) -> Element {
			let sel: Selected = use_signal(|| None);
			use_context_provider(|| sel);
			rsx! { Treemap { properties } }
		}
		let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { properties });
		dom.rebuild_in_place();
		dioxus_ssr::render(&dom)
	}

	/// The defect: price-less `Purchasing` holdings collapsed to a zero-area sliver
	/// when shown beside priced ones, so they disappeared. Every drawn tile must hold
	/// a meaningful share of the 100×100 board (>1%).
	#[test]
	fn priceless_holdings_stay_visible() {
		let tiles = Tile::layout(&mixed_book());
		for t in &tiles {
			let area = t.rect.w * t.rect.h;
			assert!(area > 100.0, "{} collapsed to {area:.4} (<1% of board) — invisible", t.name);
		}
		assert!(tiles.iter().filter(|t| t.prospect).count() == 2, "both purchasing tiles present");
		insta::assert_snapshot!("heatmap_mixed_book", render(mixed_book()));
	}

	/// The heatmap is the committed book: `Interesting` never drawn, everything else
	/// kept, empty selection draws nothing.
	#[test]
	fn query_states_drops_only_interesting() {
		use PropertyStateKind::*;
		assert_eq!(query_states(&[]), Vec::<PropertyStateKind>::new());
		assert_eq!(query_states(&[Purchased]), vec![Purchased]);
		assert_eq!(query_states(&[Purchased, Interesting, Purchasing]), vec![Purchased, Purchasing]);
	}

	#[test]
	fn treemap_tiles_tile_the_bounds() {
		let bounds = Rect { x: 0.0, y: 0.0, w: 100.0, h: 60.0 };
		let values = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
		let rects = squarify(&values, bounds);
		assert_eq!(rects.len(), values.len());

		// Areas sum to the bounds area (full coverage, no gaps/overlap budget).
		let area: f64 = rects.iter().map(|r| r.w * r.h).sum();
		assert!((area - bounds.w * bounds.h).abs() < 1.0, "covered {area}, want {}", bounds.w * bounds.h);

		let total: f64 = values.iter().sum();
		for (v, r) in values.iter().zip(&rects) {
			assert!(r.w >= 0.0 && r.h >= 0.0, "negative tile");
			assert!(r.x >= -1e-6 && r.y >= -1e-6, "tile escapes top-left");
			assert!(r.x + r.w <= bounds.w + 1e-3 && r.y + r.h <= bounds.h + 1e-3, "tile escapes bounds");
			// Area is proportional to value.
			let want = v / total * bounds.w * bounds.h;
			assert!((r.w * r.h - want).abs() < 1e-3, "area {} not ∝ value, want {want}", r.w * r.h);
		}
	}
}
