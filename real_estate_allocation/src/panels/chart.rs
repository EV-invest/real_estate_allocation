use dioxus::prelude::*;
use ev_lib::uikit::{Card, CardContent, CardDescription, CardHeader, CardTitle, Skeleton};

use crate::{app::SelectedProperty, domain::Money};

#[component]
pub fn ChartPanel() -> Element {
	let property = use_context::<SelectedProperty>();

	rsx! {
		Card { class: "flex h-[400px] flex-col overflow-hidden",
			CardHeader {
				CardTitle { class: "font-serif text-main-accent-t1", "Weekly price estimates" }
				CardDescription { "Estimated value · dimmed before purchase, dotted where stale" }
			}
			CardContent { class: "flex flex-1 flex-col gap-4",
				match &*property.read() {
					Some(Some(p)) => rsx! { ChartBody { price: p.price } },
					Some(None) => rsx! { p { class: "text-sm text-muted-foreground", "Select a property on the map." } },
					None => rsx! { Skeleton { class: "h-full w-full" } },
				}
			}
		}
	}
}

#[component]
fn ChartBody(price: Option<Money>) -> Element {
	rsx! {
		div { class: "flex items-baseline gap-3",
			match price {
				Some(p) => rsx! { span { class: "font-serif text-3xl font-semibold", "{p}" } },
				None => rsx! { span { class: "font-serif text-3xl font-semibold text-warn", "?" } },
			}
		}
		PriceChart {}
	}
}

/// Plotly price line. The axis always spans purchase→now: the owned stretch is full
/// colour, any pre-purchase estimates are dimmed (lower oklch luminosity/chroma), and
/// if the series goes stale before today a dotted line carries the last value to now.
#[component]
fn PriceChart() -> Element {
	#[cfg(target_arch = "wasm32")]
	{
		let property = use_context::<SelectedProperty>();
		use_effect(move || {
			let guard = property.read();
			let Some(Some(p)) = guard.as_ref() else { return };
			let points: Vec<(i64, f64)> = p.price_series.iter().map(|(t, v)| (t.as_millisecond(), *v)).collect();
			let (Some(first), Some(last)) = (points.first().map(|p| p.1), points.last().map(|p| p.1)) else {
				return;
			};
			let color_key = if last >= first { "up" } else { "down" };
			let purchase_ms = match p.state {
				crate::domain::PropertyState::Purchased(ts) => ts.as_millisecond() as f64,
				_ => f64::NAN,
			};
			plot_prices("rea-chart", &serde_json::to_string(&points).expect("Vec<(i64,f64)> serializes"), purchase_ms, color_key);
		});
	}
	rsx! {
		div { id: "rea-chart", class: "min-h-0 w-full flex-1" }
	}
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
function __reaLine(x, y, color) {
  return { x: x, y: y, type: 'scatter', mode: 'lines+markers',
    line: { color: color, width: 2, shape: 'spline' },
    marker: { color: color, size: 6 },
    hovertemplate: '$%{y:.1f}<extra></extra>' };
}

export function rea_plot_prices(elId, pointsJson, purchaseMs, colorKey) {
  if (!window.Plotly) { setTimeout(() => rea_plot_prices(elId, pointsJson, purchaseMs, colorKey), 200); return; }
  const el = document.getElementById(elId);
  if (!el) return;
  const pts = JSON.parse(pointsJson);
  if (!pts.length) { window.Plotly.purge(el); return; }

  const now = Date.now();
  const P = isNaN(purchaseMs) ? null : purchaseMs;
  const pal = colorKey === 'down'
    ? { full: 'oklch(0.62 0.19 25)',  dim: 'oklch(0.52 0.05 25)' }
    : { full: 'oklch(0.70 0.14 152)', dim: 'oklch(0.58 0.04 152)' };

  const ms = pts.map(p => p[0]);
  const ys = pts.map(p => p[1]);
  const lastMs = ms[ms.length - 1];
  const lastY = ys[ys.length - 1];
  const D = (arr) => arr.map(m => new Date(m));

  // Linearly interpolate the value at the purchase instant so the dim→full seam sits
  // exactly on the purchase date rather than snapping to the nearest weekly point.
  const interp = (t) => {
    if (t <= ms[0]) return ys[0];
    if (t >= lastMs) return lastY;
    for (let i = 1; i < ms.length; i++) {
      if (ms[i] >= t) { const f = (t - ms[i - 1]) / (ms[i] - ms[i - 1]); return ys[i - 1] + f * (ys[i] - ys[i - 1]); }
    }
    return lastY;
  };

  const traces = [];
  if (P !== null && P > ms[0] && P < lastMs) {
    const vp = interp(P);
    const preMs = [], preY = [], postMs = [], postY = [];
    for (let i = 0; i < ms.length; i++) {
      if (ms[i] <= P) { preMs.push(ms[i]); preY.push(ys[i]); }
      else { postMs.push(ms[i]); postY.push(ys[i]); }
    }
    preMs.push(P); preY.push(vp);
    postMs.unshift(P); postY.unshift(vp);
    traces.push(__reaLine(D(preMs), preY, pal.dim));
    traces.push(__reaLine(D(postMs), postY, pal.full));
  } else if (P !== null && P >= lastMs) {
    traces.push(__reaLine(D(ms), ys, pal.dim));
  } else {
    traces.push(__reaLine(D(ms), ys, pal.full));
  }

  // Stale tail: carry the last estimate to today as a dotted projection.
  if (lastMs < now) {
    traces.push({ x: D([lastMs, now]), y: [lastY, lastY], type: 'scatter', mode: 'lines',
      line: { color: pal.full, width: 2, dash: 'dot' }, hoverinfo: 'skip', showlegend: false });
  }

  const left = (P !== null) ? Math.min(P, ms[0]) : ms[0];
  const layout = {
    margin: { l: 56, r: 14, t: 8, b: 28 },
    paper_bgcolor: 'rgba(0,0,0,0)', plot_bgcolor: 'rgba(0,0,0,0)',
    font: { color: '#9a9486', size: 11 },
    xaxis: { type: 'date', showgrid: false, range: [new Date(left), new Date(now)] },
    yaxis: { tickprefix: '$', showgrid: true, gridcolor: 'rgba(230,225,211,0.08)', zeroline: false },
    showlegend: false,
  };
  window.Plotly.react(el, traces, layout, { displayModeBar: false, responsive: true });
}
"#)]
extern "C" {
	#[wasm_bindgen(js_name = rea_plot_prices)]
	fn plot_prices(el_id: &str, points_json: &str, purchase_ms: f64, color_key: &str);
}
