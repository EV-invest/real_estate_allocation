use dioxus::prelude::*;
use ev::uikit::{Card, CardContent, CardHeader, CardTitle, ChartConfig, ChartContainer, ChartSeries, Skeleton};

use crate::app::Selected;

#[component]
pub fn ChartPanel() -> Element {
	let selected = use_context::<Selected>();
	let property = use_resource(move || async move {
		match selected() {
			Some(id) => crate::api::get_property(id).await.ok().flatten(),
			None => None,
		}
	});

	rsx! {
		Card { class: "overflow-hidden",
			CardHeader {
				CardTitle { class: "font-serif text-main-accent-t1", "Price series" }
			}
			CardContent {
				match &*property.read() {
					Some(Some(p)) => rsx! { PriceChart { series: p.price_series.clone() } },
					Some(None) => rsx! { p { class: "text-muted-foreground text-sm", "Select a property on the map." } },
					None => rsx! { Skeleton { class: "h-48 w-full" } },
				}
			}
		}
	}
}

#[component]
fn PriceChart(series: Vec<f64>) -> Element {
	if series.is_empty() {
		return rsx! { Skeleton { class: "h-48 w-full" } };
	}

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

	let config: ChartConfig = vec![(
		"price".to_string(),
		ChartSeries {
			label: Some("Weekly estimate".to_string()),
			color: Some("var(--color-main-accent-t3)".to_string()),
		},
	)];

	rsx! {
		ChartContainer { id: "price", config,
			svg {
				class: "w-full h-48",
				view_box: "0 0 1000 200",
				preserve_aspect_ratio: "none",
				polyline {
					points,
					fill: "none",
					stroke: "var(--color-price)",
					stroke_width: "2",
				}
			}
		}
	}
}
