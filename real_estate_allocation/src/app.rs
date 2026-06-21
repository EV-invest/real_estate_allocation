use dioxus::prelude::*;

use crate::{
	dashboard::Dashboard,
	domain::{Property, PropertyId, PropertyStateKind},
};

/// The selected property, shared from the root so all panels read/write the same
/// selection. `None` until the user clicks a marker.
pub type Selected = Signal<Option<PropertyId>>;

/// Portfolio state filter, shared so the map and heatmap show the same set.
/// Defaults to `Purchased`.
pub type Filter = Signal<Vec<PropertyStateKind>>;

/// The fetched record for the current selection, resolved once at the root and
/// shared so the top bar, chart, and details panel don't each re-fetch it.
/// Outer `None` = still loading; `Some(None)` = nothing selected.
pub type SelectedProperty = Resource<Option<Property>>;

#[component]
pub fn App() -> Element {
	rsx! {
		document::Stylesheet { href: asset!("/assets/tailwind.css") }
		// Self-hosted webfonts (staged into assets/fonts by the flake, bundled via
		// `asset!`) so the dashboard renders identically offline / behind a CSP
		// instead of depending on the Google Fonts CDN. Family names match the
		// `--font-sans` / `--font-serif` token chains; variable TTFs, so one face
		// per style spans the whole weight axis.
		document::Style { {format!(
			"@font-face{{font-family:'Inter';font-style:normal;font-weight:100 900;font-display:swap;src:url('{INTER}') format('truetype')}}\
			 @font-face{{font-family:'Inter';font-style:italic;font-weight:100 900;font-display:swap;src:url('{INTER_ITALIC}') format('truetype')}}\
			 @font-face{{font-family:'Playfair Display';font-style:normal;font-weight:300 900;font-display:swap;src:url('{PLAYFAIR}') format('truetype')}}\
			 @font-face{{font-family:'Playfair Display';font-style:italic;font-weight:300 900;font-display:swap;src:url('{PLAYFAIR_ITALIC}') format('truetype')}}",
			INTER = asset!("/assets/fonts/Inter.ttf"),
			INTER_ITALIC = asset!("/assets/fonts/Inter-Italic.ttf"),
			PLAYFAIR = asset!("/assets/fonts/PlayfairDisplay.ttf"),
			PLAYFAIR_ITALIC = asset!("/assets/fonts/PlayfairDisplay-Italic.ttf"),
		)} }
		Router::<Route> {}
	}
}
/// Two surfaces off one binary: the full dashboard at `/`, and the iframe-only
/// marketing overview at `/embed/overview`. The split lives in the router so the
/// embed carries none of the dashboard's shell, contexts, or Maps script.
#[derive(Clone, PartialEq, Routable)]
enum Route {
	#[route("/")]
	Home {},
	#[route("/embed/overview")]
	EmbedOverview {},
}

#[component]
fn Home() -> Element {
	let selected: Selected = use_signal(initial_selection);
	use_context_provider(|| selected);

	let filter: Filter = use_signal(initial_filter);
	use_context_provider(|| filter);

	// Mirror the state filter into `?selection=` so a shared link restores the set.
	#[cfg(target_arch = "wasm32")]
	use_effect(move || {
		let csv = filter().iter().map(AsRef::as_ref).collect::<Vec<&str>>().join(",");
		crate::map::sync_selection(&csv);
	});

	let property: SelectedProperty = use_resource(move || async move {
		match selected() {
			Some(id) => crate::api::get_property(id).await.ok().flatten(),
			None => None,
		}
	});
	use_context_provider(|| property);

	// Maps JS key is server-side config; fetch it so the loader `<script>` can be
	// emitted with the right key (Maps JS keys are public, restricted by referrer).
	let maps_key = use_resource(crate::api::maps_api_key);
	let maps_src = match &*maps_key.read() {
		Some(Ok(key)) if !key.is_empty() => Some(format!("https://maps.googleapis.com/maps/api/js?key={key}&libraries=places&v=weekly&callback=__reaMapsReady")),
		Some(Ok(_)) => {
			dioxus::logger::tracing::error!("maps_api_key resolved empty — set it in config; the map will not load");
			None
		}
		Some(Err(e)) => {
			dioxus::logger::tracing::error!(%e, "maps_api_key fetch failed — the map will not load");
			None
		}
		None => None, // still loading
	};

	// The dock overlay positions panels from live DOM measurements and writes dynamic
	// `style` attributes; under SSR hydration that mismatches the server DOM and the
	// Dioxus interpreter aborts the mutation batch — panels stay `visibility:hidden`,
	// blank and click-eating. An interactive layout has nothing to pre-render, so we
	// render it client-only (effects don't run during SSR, so `ready` flips post-mount).
	let mut ready = use_signal(|| false);
	use_effect(move || ready.set(true));

	rsx! {
		if ready() {
			if let Some(src) = maps_src {
				document::Script { src, defer: true }
			}
			document::Script { src: "https://cdn.plot.ly/plotly-basic-2.35.2.min.js", defer: true }
			Dashboard {}
		}
	}
}

#[component]
fn EmbedOverview() -> Element {
	rsx! {
		crate::embed::Overview {}
	}
}
/// Deep-link support: read `?property=<uuid>` so a shared URL opens preselected.
fn initial_selection() -> Option<PropertyId> {
	#[cfg(target_arch = "wasm32")]
	{
		crate::domain::parse_property_id(&crate::map::url_property()).ok()
	}
	#[cfg(not(target_arch = "wasm32"))]
	{
		None
	}
}

/// Deep-link support: read `?selection=purchased,purchasing`, default `Purchased`.
fn initial_filter() -> Vec<PropertyStateKind> {
	#[cfg(target_arch = "wasm32")]
	{
		// URL is user-editable: silently drop tokens that don't name a state, and
		// fall back to the default set if nothing valid remains.
		let states: Vec<_> = crate::map::url_selection().split(',').filter_map(|s| s.trim().parse().ok()).collect();
		if states.is_empty() { vec![PropertyStateKind::Purchased] } else { states }
	}
	#[cfg(not(target_arch = "wasm32"))]
	{
		vec![PropertyStateKind::Purchased]
	}
}
