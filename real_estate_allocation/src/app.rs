use dioxus::prelude::*;

use crate::{dashboard::Dashboard, domain::PropertyId};

/// The selected property, shared from the root so all four panels read/write the
/// same selection. `None` until the user clicks a marker.
pub type Selected = Signal<Option<PropertyId>>;

#[component]
pub fn App() -> Element {
	let selected: Selected = use_signal(|| None::<PropertyId>);
	use_context_provider(|| selected);

	// Maps JS key is server-side config; fetch it so the loader `<script>` can be
	// emitted with the right key (Maps JS keys are public, restricted by referrer).
	let maps_key = use_resource(crate::api::maps_api_key);
	let maps_src = match &*maps_key.read() {
		Some(Ok(key)) if !key.is_empty() => Some(format!("https://maps.googleapis.com/maps/api/js?key={key}&callback=__reaMapsReady")),
		_ => None,
	};

	rsx! {
		document::Stylesheet { href: asset!("/assets/tailwind.css") }
		document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
		document::Link {
			rel: "stylesheet",
			href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Playfair+Display:wght@600;700&display=swap",
		}
		if let Some(src) = maps_src {
			document::Script { src, defer: true }
		}
		Dashboard {}
	}
}
