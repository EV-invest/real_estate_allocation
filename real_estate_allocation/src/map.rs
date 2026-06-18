//! Isolated Google-Maps module — the ONLY file that touches the JS Maps API.
//! Everything except `MapPanel` is `cfg(wasm32)`-private; the server/native build
//! renders a placeholder so the inline-JS extern is never linked off-target.

use dioxus::prelude::*;
use ev::uikit::{Card, CardContent, CardHeader, CardTitle, Skeleton, ToggleGroup, ToggleGroupItem};

use crate::{app::Selected, domain::PropertyState};

#[component]
pub fn MapPanel() -> Element {
	let selected = use_context::<Selected>();

	// State filter for the map; default = Purchased only.
	let filter = use_signal(|| vec![PropertyState::Purchased]);

	let properties = use_resource(move || {
		let states = filter();
		async move { crate::api::list_properties(Some(states)).await.unwrap_or_default() }
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
		Card { class: "overflow-hidden flex flex-col",
			CardHeader { class: "flex-row items-center justify-between gap-2",
				CardTitle { class: "font-serif text-main-accent-t1", "Portfolio map" }
				StateFilter { filter }
			}
			CardContent { class: "flex-1 relative p-0",
				div { id: "rea-map", class: "absolute inset-0" }
				if properties.read().is_none() {
					Skeleton { class: "absolute inset-0" }
				}
			}
		}
	}
}

#[component]
fn StateFilter(filter: Signal<Vec<PropertyState>>) -> Element {
	let toggle = move |state: PropertyState| {
		move |pressed: bool| {
			let mut cur = filter();
			if pressed {
				if !cur.contains(&state) {
					cur.push(state);
				}
			} else {
				cur.retain(|s| *s != state);
			}
			filter.set(cur);
		}
	};
	let cur = filter();

	rsx! {
		ToggleGroup {
			ToggleGroupItem {
				pressed: cur.contains(&PropertyState::Purchased),
				on_pressed_change: toggle(PropertyState::Purchased),
				"Purchased"
			}
			ToggleGroupItem {
				pressed: cur.contains(&PropertyState::Interesting),
				on_pressed_change: toggle(PropertyState::Interesting),
				"Interesting"
			}
			ToggleGroupItem {
				pressed: cur.contains(&PropertyState::Purchasing),
				on_pressed_change: toggle(PropertyState::Purchasing),
				"Purchasing"
			}
		}
	}
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
let __reaMap = null;
let __reaMarkers = {};

window.__reaMapsReady = function () { window.__reaMapsLoaded = true; };

function __reaColor(state) {
  switch (state) {
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
    __reaMap = new google.maps.Map(el, { center: { lat: 25, lng: 0 }, zoom: 2, disableDefaultUI: true, backgroundColor: '#070d18' });
  }
  const seen = {};
  props.forEach(p => {
    const id = p.id;
    seen[id] = true;
    const pos = { lat: p.coords.lat, lng: p.coords.lng };
    const color = __reaColor(p.state);
    const scale = id === selectedId ? 11 : 7;
    const icon = { path: google.maps.SymbolPath.CIRCLE, fillColor: color, fillOpacity: 1, strokeColor: '#070d18', strokeWeight: 2, scale: scale };
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
"#)]
extern "C" {
	#[wasm_bindgen(js_name = rea_render_markers)]
	fn render_markers(el_id: &str, props_json: &str, selected_id: &str);
	fn rea_on_select(cb: &wasm_bindgen::closure::Closure<dyn FnMut(String)>);
	#[wasm_bindgen(js_name = rea_sync_url)]
	fn sync_url(selected_id: &str);
	#[wasm_bindgen(js_name = rea_url_property)]
	pub fn url_property() -> String;
}
