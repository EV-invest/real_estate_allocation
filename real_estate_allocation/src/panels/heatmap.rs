use dioxus::prelude::*;
use ev::uikit::{Card, CardContent, CardDescription, CardHeader, CardTitle, Skeleton};

use crate::{
	app::Selected,
	domain::{Property, PropertyId},
};

/// Portfolio heatmap: a treemap of every holding, area ∝ price, colour ∝ recent
/// value change, with the selected property highlighted and each tile clickable.
#[component]
pub fn PortfolioHeatmap() -> Element {
	// The whole portfolio (defaults to Purchased holdings), independent of the
	// per-property selection.
	let properties = use_resource(|| async move { crate::api::list_properties(None).await.unwrap_or_default() });

	rsx! {
		Card {
			CardHeader { class: "flex flex-row items-start justify-between gap-2",
				div {
					CardTitle { class: "font-serif text-main-accent-t1", "Portfolio heatmap" }
					CardDescription { "Estimated value change across all holdings · last 30 days" }
				}
				Legend {}
			}
			CardContent {
				match &*properties.read() {
					None => rsx! { Skeleton { class: "h-[300px] w-full" } },
					Some(list) if list.is_empty() => rsx! {
						div { class: "flex h-[300px] items-center justify-center text-sm text-muted-foreground",
							"No holdings to display."
						}
					},
					Some(list) => rsx! { Treemap { properties: list.clone() } },
				}
			}
		}
	}
}

#[component]
fn Treemap(properties: Vec<Property>) -> Element {
	let mut selected = use_context::<Selected>();
	let values: Vec<f64> = properties.iter().map(|p| p.price.amount().max(1.0)).collect();
	let rects = squarify(&values, Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 });

	rsx! {
		div { class: "relative h-[300px] w-full overflow-hidden rounded-lg bg-main-surface",
			for (i , p) in properties.iter().enumerate() {
				{
					let r = rects[i];
					let pid = p.id;
					let change = mock_change(pid);
					let is_sel = selected() == Some(pid);
					let ring = if is_sel { "ring-2 ring-main-accent-t1 z-10" } else { "" };
					rsx! {
						button {
							key: "{i}",
							class: "absolute flex flex-col justify-between overflow-hidden rounded-md p-2.5 text-left transition-[filter] hover:brightness-110 {ring}",
							style: "left:calc({r.x:.4}% + 2px);top:calc({r.y:.4}% + 2px);width:calc({r.w:.4}% - 4px);height:calc({r.h:.4}% - 4px);background-color:{heat_color(change)}",
							onclick: move |_| selected.set(Some(pid)),
							div { class: "flex min-w-0 flex-col gap-0.5",
								span { class: "truncate text-sm font-semibold text-white", "{p.name}" }
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

#[component]
fn Legend() -> Element {
	rsx! {
		div { class: "flex items-center gap-2 text-xs text-muted-foreground",
			span { class: "size-3 rounded-sm", style: "background-color:{heat_color(-4.0)}" }
			span { "Loss" }
			span { class: "size-3 rounded-sm", style: "background-color:{heat_color(6.0)}" }
			span { "Gain" }
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

#[derive(Clone, Copy)]
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
