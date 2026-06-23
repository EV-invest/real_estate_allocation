//! Isolated Google-Maps module — the ONLY file that touches the JS Maps API.
//! Everything except `MapPanel` is `cfg(wasm32)`-private; the server/native build
//! renders a placeholder so the inline-JS extern is never linked off-target.

use dioxus::prelude::*;
use ev_lib::uikit::{Card, CardContent, Skeleton};

use crate::{app::Selected, domain::PropertyStateKind};

#[component]
pub fn MapPanel() -> Element {
	let selected = use_context::<Selected>();

	// Shared with the heatmap so both show the same set.
	let filter = use_context::<crate::app::Filter>();

	let properties = use_resource(move || {
		let states = filter();
		async move {
			crate::api::list_properties(Some(states))
				.await
				.inspect_err(|e| dioxus::logger::tracing::error!(%e, "map: list_properties failed"))
				.unwrap_or_default()
		}
	});

	// Push the property list + current selection into the JS layer whenever either
	// changes. No-op on the server build.
	#[cfg(target_arch = "wasm32")]
	{
		use_effect(move || {
			// Read both reactive sources *inside* the effect so it re-runs whenever
			// the property list or the selection changes.
			let sel = selected().map(|id| id.raw().to_string()).unwrap_or_default();
			sync_url(&sel);
			if let Some(props) = properties.read().as_ref() {
				let json = serde_json::to_string(props).unwrap_or_else(|_| "[]".into());
				render_markers("rea-map", &json, &sel);
			}
		});

		// JS→Rust marker-click bridge, installed once.
		let mut selected = selected;
		use_hook(move || {
			let cb = wasm_bindgen::closure::Closure::<dyn FnMut(String)>::new(move |id: String| {
				if let Ok(pid) = crate::domain::parse_property_id(&id) {
					selected.set(Some(pid));
				}
			});
			rea_on_select(&cb);
			// Leak so the closure outlives this scope; the page owns it for its lifetime.
			cb.forget();
		});
	}
	#[cfg(not(target_arch = "wasm32"))]
	let _ = (selected, &properties);

	rsx! {
		// h-full so the map fills (and resizes with) its dock pane; the tab already labels it,
		// so the redundant card header is gone — only the live state filter survives, overlaid.
		Card { class: "flex h-full flex-col overflow-hidden",
			CardContent { class: "flex-1 relative p-0",
				div { id: "rea-map", class: "absolute inset-0" }
				div { class: "absolute right-3 top-3 z-10",
					StateFilter { filter }
				}
				if properties.read().is_none() {
					Skeleton { class: "absolute inset-0" }
				}
			}
		}
	}
}

/// Filter chips — plain `<button>`s. An active chip is filled with its state's own
/// marker colour (so it reads at a glance and matches the map pins); inactive ones
/// are a muted outline.
//TODO: switch to the lib's `ToggleGroup` once its interface is fixed.
#[component]
fn StateFilter(filter: Signal<Vec<PropertyStateKind>>) -> Element {
	let cur = filter();
	rsx! {
		div { class: "flex items-center gap-1.5",
			for state in [PropertyStateKind::Purchased, PropertyStateKind::Purchasing, PropertyStateKind::Interesting] {
				{
					let on = cur.contains(&state);
					// On: filled with the state colour + dark text. Off: outlined + muted.
					let active = match state {
						PropertyStateKind::Purchased => "bg-main-accent-t2",
						PropertyStateKind::Interesting => "bg-main-accent-t3",
						PropertyStateKind::Purchasing => "bg-main-accent-t4",
					};
					let cls = if on {
						format!("h-7 rounded-md px-2.5 text-xs font-semibold text-main-black transition hover:brightness-110 {active}")
					} else {
						"h-7 rounded-md border border-border bg-main-surface px-2.5 text-xs font-medium text-muted-foreground transition hover:border-main-mist/40 hover:text-foreground".to_string()
					};
					rsx! {
						button {
							key: "{state}",
							r#type: "button",
							class: cls,
							"aria-pressed": on,
							onclick: move |_| {
								let mut c = filter();
								if on {
									c.retain(|s| *s != state);
								} else {
									c.push(state);
								}
								filter.set(c);
							},
							"{state}"
						}
					}
				}
			}
		}
	}
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
let __reaMap = null;
let __reaMarkers = {};
let __reaFitted = false;

window.__reaMapsReady = function () { window.__reaMapsLoaded = true; };

function __reaColor(state) {
  // `Purchased` serialises as `{Purchased: <ts>}`; the unit variants as plain strings.
  const kind = (typeof state === 'string') ? state : Object.keys(state)[0];
  switch (kind) {
    case 'Purchased': return '#2e9e5b';
    case 'Interesting': return '#f2c94c';
    case 'Purchasing': return '#e58aae';
    default: return '#e6e1d3';
  }
}

export function rea_render_markers(elId, propsJson, selectedId) {
  if (!window.google || !window.google.maps) { setTimeout(() => rea_render_markers(elId, propsJson, selectedId), 300); return; }
  const el = document.getElementById(elId);
  if (!el) return;
  const props = JSON.parse(propsJson);
  if (!__reaMap) {
    __reaMap = new google.maps.Map(el, { center: { lat: 13.78, lng: 109.22 }, zoom: 12, disableDefaultUI: true, backgroundColor: '#070d18' });
  }
  const seen = {};
  const bounds = new google.maps.LatLngBounds();
  props.forEach(p => {
    // Coords are resolved + cached server-side; an unresolved place just has no pin.
    if (!p.coords) return;
    const id = p.id;
    seen[id] = true;
    const pos = { lat: p.coords.lat, lng: p.coords.lng };
    const color = __reaColor(p.state);
    const scale = id === selectedId ? 11 : 7;
    const icon = { path: google.maps.SymbolPath.CIRCLE, fillColor: color, fillOpacity: 1, strokeColor: '#070d18', strokeWeight: 2, scale: scale };
    bounds.extend(pos);
    let m = __reaMarkers[id];
    if (!m) {
      m = new google.maps.Marker({ position: pos, map: __reaMap, icon: icon });
      m.addListener('click', () => { if (window.__reaSelect) window.__reaSelect(id); });
      __reaMarkers[id] = m;
    } else {
      m.setPosition(pos); m.setIcon(icon); m.setMap(__reaMap);
    }
  });
  Object.keys(__reaMarkers).forEach(id => { if (!seen[id]) { __reaMarkers[id].setMap(null); delete __reaMarkers[id]; } });
  // Fit once, so later selection re-renders don't yank the viewport around.
  if (!__reaFitted && !bounds.isEmpty()) { __reaMap.fitBounds(bounds, 48); __reaFitted = true; }
}

export function rea_on_select(cb) { window.__reaSelect = cb; }

export function rea_sync_url(selectedId) {
  const url = new URL(window.location.href);
  if (selectedId) { url.searchParams.set('property', selectedId); }
  else { url.searchParams.delete('property'); }
  window.history.replaceState({}, '', url);
}

export function rea_url_property() {
  return new URL(window.location.href).searchParams.get('property') || '';
}

export function rea_sync_selection(csv) {
  const url = new URL(window.location.href);
  if (csv) { url.searchParams.set('selection', csv); }
  else { url.searchParams.delete('selection'); }
  window.history.replaceState({}, '', url);
}

export function rea_url_selection() {
  return new URL(window.location.href).searchParams.get('selection') || '';
}
"#)]
extern "C" {
	#[wasm_bindgen(js_name = rea_render_markers)]
	fn render_markers(el_id: &str, props_json: &str, selected_id: &str);
	fn rea_on_select(cb: &wasm_bindgen::closure::Closure<dyn FnMut(String)>);
	#[wasm_bindgen(js_name = rea_sync_url)]
	fn sync_url(selected_id: &str);
	#[wasm_bindgen(js_name = rea_url_property)]
	pub fn url_property() -> String;
	#[wasm_bindgen(js_name = rea_sync_selection)]
	pub fn sync_selection(csv: &str);
	#[wasm_bindgen(js_name = rea_url_selection)]
	pub fn url_selection() -> String;
}
