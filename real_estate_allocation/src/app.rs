use dioxus::prelude::*;

use crate::{
	dashboard::Dashboard,
	domain::{Property, PropertyId},
};

/// The selected property, shared from the root so all panels read/write the same
/// selection. `None` until the user clicks a marker.
pub type Selected = Signal<Option<PropertyId>>;

/// The fetched record for the current selection, resolved once at the root and
/// shared so the top bar, chart, and details panel don't each re-fetch it.
/// Outer `None` = still loading; `Some(None)` = nothing selected.
pub type SelectedProperty = Resource<Option<Property>>;

#[component]
pub fn App() -> Element {
	rsx! {
		document::Stylesheet { href: asset!("/assets/tailwind.css") }
		document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
		document::Link {
			rel: "stylesheet",
			href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Playfair+Display:wght@600;700&display=swap",
		}
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
		_ => None,
	};

	rsx! {
		if let Some(src) = maps_src {
			document::Script { src, defer: true }
		}
		Dashboard {}
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
