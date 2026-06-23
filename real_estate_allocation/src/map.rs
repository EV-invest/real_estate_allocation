//! Isolated Google-Maps module — the ONLY file that touches the JS Maps API.
//! Everything except `MapPanel` is `cfg(wasm32)`-private; the server/native build
//! renders a placeholder so the inline-JS extern is never linked off-target.

use dioxus::prelude::*;
use ev_lib::uikit::{Card, CardContent, Skeleton};

use crate::{
	app::{SelectedAppt, SelectedBuilding},
	domain::PropertyStateKind,
};

#[component]
pub fn MapPanel() -> Element {
	let selected = use_context::<SelectedBuilding>();
	let appt = use_context::<SelectedAppt>();

	// Shared with the heatmap so both show the same set.
	let filter = use_context::<crate::app::Filter>();

	let buildings = use_resource(move || {
		let states = filter();
		async move {
			crate::api::list_buildings(Some(states))
				.await
				.inspect_err(|e| dioxus::logger::tracing::error!(%e, "map: list_buildings failed"))
				.unwrap_or_default()
		}
	});

	// Push the building list + current selection into the JS layer whenever any
	// changes. No-op on the server build.
	#[cfg(target_arch = "wasm32")]
	{
		use_effect(move || {
			// Read every reactive source *inside* the effect so it re-runs on any change.
			let sel = selected().map(|id| id.raw().to_string()).unwrap_or_default();
			let appt = appt().map(|n| n.to_string()).unwrap_or_default();
			sync_url(&sel, &appt);
			if let Some(list) = buildings.read().as_ref() {
				let json = serde_json::to_string(list).unwrap_or_else(|_| "[]".into());
				render_markers("rea-map", &json, &sel);
			}
		});

		// JS→Rust marker-click bridge, installed once. Selecting a building clears any
		// apartment drilled into the previous one.
		let mut selected = selected;
		let mut appt = appt;
		use_hook(move || {
			let cb = wasm_bindgen::closure::Closure::<dyn FnMut(String)>::new(move |id: String| {
				if let Ok(bid) = crate::domain::parse_building_id(&id) {
					selected.set(Some(bid));
					appt.set(None);
				}
			});
			rea_on_select(&cb);
			// Leak so the closure outlives this scope; the page owns it for its lifetime.
			cb.forget();
		});
	}
	#[cfg(not(target_arch = "wasm32"))]
	let _ = (selected, appt, &buildings);

	rsx! {
		// h-full so the map fills (and resizes with) its dock pane; the tab already labels it,
		// so the redundant card header is gone — only the live state filter survives, overlaid.
		Card { class: "flex h-full flex-col overflow-hidden",
			CardContent { class: "flex-1 relative p-0",
				div { id: "rea-map", class: "absolute inset-0" }
				div { class: "absolute right-3 top-3 z-10",
					StateFilter { filter }
				}
				if buildings.read().is_none() {
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

function __reaColor(apartments) {
  // A building's pin colour is its strongest portfolio relationship across lots:
  // Purchased > Purchasing > Interesting. A lot's `Purchased` serialises as
  // `{Purchased: <ts>}`; the other statuses as plain strings.
  let purchasing = false, interesting = false;
  for (const a of (apartments || [])) {
    const kind = (typeof a.status === 'string') ? a.status : Object.keys(a.status)[0];
    if (kind === 'Purchased') return '#2e9e5b';
    if (kind === 'Purchasing') purchasing = true;
    else if (kind === 'Interesting') interesting = true;
  }
  if (purchasing) return '#e58aae';
  if (interesting) return '#f2c94c';
  return '#e6e1d3';
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
    const color = __reaColor(p.apartments);
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

export function rea_on_keynav(cb) {
  window.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowLeft') cb(-1);
    else if (e.key === 'ArrowRight') cb(1);
  });
}

export function rea_sync_url(buildingId, appt) {
  const url = new URL(window.location.href);
  if (buildingId) { url.searchParams.set('building', buildingId); }
  else { url.searchParams.delete('building'); }
  if (appt) { url.searchParams.set('appt', appt); }
  else { url.searchParams.delete('appt'); }
  window.history.replaceState({}, '', url);
}

export function rea_url_building() {
  return new URL(window.location.href).searchParams.get('building') || '';
}

export function rea_url_appt() {
  return new URL(window.location.href).searchParams.get('appt') || '';
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
	#[wasm_bindgen(js_name = rea_on_keynav)]
	pub fn on_keynav(cb: &wasm_bindgen::closure::Closure<dyn FnMut(i32)>);
	#[wasm_bindgen(js_name = rea_sync_url)]
	fn sync_url(building_id: &str, appt: &str);
	#[wasm_bindgen(js_name = rea_url_building)]
	pub fn url_building() -> String;
	#[wasm_bindgen(js_name = rea_url_appt)]
	pub fn url_appt() -> String;
	#[wasm_bindgen(js_name = rea_sync_selection)]
	pub fn sync_selection(csv: &str);
	#[wasm_bindgen(js_name = rea_url_selection)]
	pub fn url_selection() -> String;
}
