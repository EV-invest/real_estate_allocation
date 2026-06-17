use dioxus::prelude::*;
use ev::uikit::{Badge, BadgeVariant, Card, CardContent, CardHeader, CardTitle, Separator, Skeleton};

use crate::{
	app::Selected,
	domain::{Property, PropertyState},
};

#[component]
pub fn DetailsPanel() -> Element {
	let selected = use_context::<Selected>();
	let property = use_resource(move || async move {
		match selected() {
			Some(id) => crate::api::get_property(id).await.ok().flatten(),
			None => None,
		}
	});

	rsx! {
		Card { class: "overflow-y-auto",
			CardHeader {
				CardTitle { class: "font-serif text-main-accent-t1", "Deal details" }
			}
			CardContent {
				match &*property.read() {
					Some(Some(p)) => rsx! { Details { property: p.clone() } },
					Some(None) => rsx! { p { class: "text-muted-foreground text-sm", "No property selected." } },
					None => rsx! { Skeleton { class: "h-48 w-full" } },
				}
			}
		}
	}
}

#[component]
fn Details(property: Property) -> Element {
	let (badge_variant, badge_label) = match property.state {
		PropertyState::Purchased => (BadgeVariant::Success, "Purchased"),
		PropertyState::Interesting => (BadgeVariant::Outline, "Interesting"),
		PropertyState::Purchasing => (BadgeVariant::Secondary, "Purchasing"),
	};

	rsx! {
		div { class: "flex flex-col gap-3",
			div { class: "flex items-center justify-between gap-2",
				span { class: "font-mono text-2xl text-main-accent-t3", "${property.price.amount():.0}" }
				Badge { variant: badge_variant, "{badge_label}" }
			}

			a {
				href: "{property.research_url.as_str()}",
				target: "_blank",
				rel: "noopener noreferrer",
				class: "text-main-accent-t1 underline-offset-4 hover:underline text-sm w-fit",
				"Open research ↗"
			}

			if let Some(terms) = property.terms.as_ref() {
				Separator {}
				Section { title: "Terms", "{terms}" }
			}

			if let Some(deal) = property.deal.as_ref() {
				Separator {}
				Section { title: "Deal structure",
					p { "Equity {deal.equity_pct:.0}% / Debt {deal.debt_pct:.0}%" }
					if let Some(notes) = deal.notes.as_ref() {
						p { class: "text-muted-foreground", "{notes}" }
					}
				}
			}

			if let Some(loan) = property.loan.as_ref() {
				Separator {}
				Section { title: "Loan",
					p { "{loan.rate_pct:.2}% over {loan.term_years}y — {loan.lender}" }
				}
			}

			if let Some(reasoning) = property.additional_reasoning.as_ref() {
				Separator {}
				Section { title: "Reasoning", "{reasoning}" }
			}
		}
	}
}

#[component]
fn Section(title: String, children: Element) -> Element {
	rsx! {
		div { class: "flex flex-col gap-1",
			span { class: "text-xs uppercase tracking-wide text-main-accent-t1", "{title}" }
			div { class: "text-sm", {children} }
		}
	}
}
