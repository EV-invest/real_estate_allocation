use dioxus::prelude::*;
use ev::uikit::{Badge, BadgeVariant};

use crate::{
	app::SelectedProperty,
	domain::{Property, PropertyState},
};

/// Persistent page header: breadcrumb, price headline, state badge, and the one
/// real action we have (open the research link). The host micro-frontend owns
/// the surrounding shell/nav, so there is intentionally no sidebar here.
#[component]
pub fn TopBar() -> Element {
	let property = use_context::<SelectedProperty>();

	rsx! {
		header { class: "sticky top-0 z-20 border-b border-border bg-background/90 backdrop-blur",
			// Inner column shares the body's max-width so the header aligns with the
			// content beneath it, while the border/background stay full-bleed.
			div { class: "mx-auto flex w-full max-w-[1200px] flex-col gap-2 px-6 py-5 lg:px-8",
				match &*property.read() {
					Some(Some(p)) => rsx! { Loaded { property: p.clone() } },
					Some(None) => rsx! { Empty {} },
					None => rsx! {
						div { class: "h-4 w-40 animate-pulse rounded bg-accent" }
						div { class: "h-8 w-64 animate-pulse rounded bg-accent" }
					},
				}
			}
		}
	}
}

#[component]
fn Loaded(property: Property) -> Element {
	let (variant, label) = badge_for(property.state);
	rsx! {
		Breadcrumb { tail: property.name.clone() }
		div { class: "flex flex-wrap items-center justify-between gap-3",
			div { class: "flex items-center gap-3",
				h1 { class: "font-serif text-3xl font-semibold tracking-tight", "{property.name}" }
				Badge { variant, "{label}" }
				span { class: "text-sm font-medium text-muted-foreground", "{property.price}" }
			}
			a {
				href: "{property.research_url.as_str()}",
				target: "_blank",
				rel: "noopener noreferrer",
				class: "inline-flex h-9 items-center justify-center gap-1.5 rounded-md border border-border bg-transparent px-4 text-sm font-medium shadow-xs transition-colors hover:bg-accent hover:text-accent-foreground",
				"Open research ↗"
			}
		}
	}
}

#[component]
fn Empty() -> Element {
	rsx! {
		Breadcrumb { tail: "Overview".to_string() }
		h1 { class: "font-serif text-3xl font-semibold tracking-tight text-muted-foreground", "Select a property" }
	}
}

#[component]
fn Breadcrumb(tail: String) -> Element {
	rsx! {
		nav { class: "flex items-center gap-2 text-sm text-muted-foreground",
			span { "Properties" }
			span { "›" }
			span { class: "text-foreground", "{tail}" }
		}
	}
}

fn badge_for(state: PropertyState) -> (BadgeVariant, &'static str) {
	match state {
		PropertyState::Purchased => (BadgeVariant::Success, "Purchased"),
		PropertyState::Interesting => (BadgeVariant::Outline, "Interesting"),
		PropertyState::Purchasing => (BadgeVariant::Secondary, "Purchasing"),
	}
}
