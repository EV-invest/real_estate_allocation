use dioxus::prelude::*;
use ev::uikit::{Card, CardContent, CardDescription, CardHeader, CardTitle, Skeleton};

use crate::{app::SelectedProperty, domain::Money};

#[component]
pub fn ChartPanel() -> Element {
	let property = use_context::<SelectedProperty>();

	rsx! {
		Card { class: "flex h-[400px] flex-col overflow-hidden",
			CardHeader {
				CardTitle { class: "font-serif text-main-accent-t1", "Weekly price estimates" }
				CardDescription { "Estimated value since acquisition" }
			}
			CardContent { class: "flex flex-1 flex-col gap-4",
				match &*property.read() {
					Some(Some(p)) => rsx! { ChartBody { series: p.price_series.clone(), price: p.price } },
					Some(None) => rsx! { p { class: "text-sm text-muted-foreground", "Select a property on the map." } },
					None => rsx! { Skeleton { class: "h-full w-full" } },
				}
			}
		}
	}
}

#[component]
fn ChartBody(series: Vec<f64>, price: Money) -> Element {
	if series.is_empty() {
		return rsx! { Skeleton { class: "h-full w-full" } };
	}
	let first = series.first().copied().unwrap_or(0.0);
	let last = series.last().copied().unwrap_or(0.0);
	let delta = if first.abs() > f64::EPSILON { (last / first - 1.0) * 100.0 } else { 0.0 };
	let up = delta >= 0.0;
	let delta_class = if up { "text-main-accent-t2" } else { "text-destructive" };
	let arrow = if up { "▲" } else { "▼" };

	rsx! {
		div { class: "flex items-baseline gap-3",
			span { class: "font-serif text-3xl font-semibold", "{price}" }
			span { class: "text-sm font-semibold {delta_class}", "{arrow} {delta:+.1}%" }
		}
		PriceChart { series, up }
		div { class: "flex justify-between text-xs text-muted-foreground",
			for l in ["Mar", "Apr", "May", "Jun", "Now"] {
				span { "{l}" }
			}
		}
	}
}

#[component]
fn PriceChart(series: Vec<f64>, up: bool) -> Element {
	let min = series.iter().copied().fold(f64::INFINITY, f64::min);
	let max = series.iter().copied().fold(f64::NEG_INFINITY, f64::max);
	let span = (max - min).max(f64::EPSILON);
	let n = series.len();
	const W: f64 = 1000.0;
	const H: f64 = 200.0;

	let points = series
		.iter()
		.enumerate()
		.map(|(i, &v)| {
			let x = (i as f64 / (n - 1).max(1) as f64) * W;
			// Flip y so higher prices sit higher on screen.
			let y = H - ((v - min) / span) * H;
			format!("{x:.2},{y:.2}")
		})
		.collect::<Vec<_>>()
		.join(" ");
	// Close the line back down to the baseline so the area under it can be filled.
	let area = format!("0,{H:.0} {points} {W:.0},{H:.0}");
	let color = if up { "var(--color-main-accent-t2)" } else { "var(--color-destructive)" };

	rsx! {
		svg {
			class: "min-h-0 w-full flex-1",
			view_box: "0 0 1000 200",
			preserve_aspect_ratio: "none",
			polygon { points: area, fill: color, fill_opacity: "0.12" }
			polyline {
				points,
				fill: "none",
				stroke: color,
				stroke_width: "2.5",
				stroke_linejoin: "round",
				stroke_linecap: "round",
			}
		}
	}
}
