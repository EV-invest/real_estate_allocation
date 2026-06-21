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
	// Shares the map's `?selection=` filter so both react to the same set. Owned tiles
	// are solid; prospects (interesting / purchasing) are striped over the heat fill.
	let filter = use_context::<crate::app::Filter>();
	let properties = use_resource(move || {
		let states = filter();
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
		// Unknown price → minimal tile area rather than dropping the holding.
		let values: Vec<f64> = properties.iter().map(|p| p.price.map_or(1.0, |m| m.amount()).max(1.0)).collect();
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
	use super::*;

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
