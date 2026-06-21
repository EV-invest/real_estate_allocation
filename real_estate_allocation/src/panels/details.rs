use dioxus::prelude::*;
use ev_lib::uikit::{Card, CardContent, CardDescription, CardHeader, CardTitle, Skeleton, Tooltip, TooltipContent, TooltipTrigger};

use crate::{
	app::SelectedProperty,
	domain::{ConstructionStatus, Property},
};

#[component]
pub fn DetailsPanel() -> Element {
	let property = use_context::<SelectedProperty>();

	rsx! {
		Card { class: "flex h-full flex-col",
			CardHeader {
				CardTitle { class: "font-serif text-main-accent-t1", "Deal terms & structure" }
				CardDescription { "Pricing, leverage, and return profile" }
			}
			CardContent { class: "flex-1 overflow-y-auto",
				match &*property.read() {
					Some(Some(p)) => rsx! { Details { property: p.clone() } },
					Some(None) => rsx! { p { class: "text-sm text-muted-foreground", "Select a property to see its terms." } },
					None => rsx! { Skeleton { class: "h-48 w-full" } },
				}
			}
		}
	}
}

#[component]
fn Details(property: Property) -> Element {
	rsx! {
		div { class: "flex flex-col",
			match property.price {
				Some(p) => rsx! { Kv { label: "Price", value_class: "text-main-accent-t3", "{p}" } },
				None => rsx! { Kv { label: "Price", value_class: "text-warn", "?" } },
			}

			if let Some(dev) = property.developer.as_ref() {
				DeveloperKv { name: dev.clone() }
			}

			Kv {
				label: "Construction",
				value_class: match property.construction {
					ConstructionStatus::Completed => "text-main-accent-t2",
					ConstructionStatus::UnderConstruction => "text-warn",
				},
				"{property.construction}"
			}

			if let Some(deal) = property.deal.as_ref() {
				Kv { label: "Equity / Debt", "{deal.equity_pct:.0}% / {deal.debt_pct:.0}%" }
			}

			if let Some(loan) = property.loan.as_ref() {
				Kv { label: "Loan rate", value_class: "text-main-accent-t1", "{loan.rate_pct:.2}%" }
				Kv { label: "Term", "{loan.term_years} yr" }
				Kv { label: "Lender", "{loan.lender}" }
			}

			if let Some(terms) = property.terms.as_ref() {
				Note { label: "Terms", "{terms}" }
			}

			if let Some(notes) = property.deal.as_ref().and_then(|d| d.notes.as_ref()) {
				Note { label: "Deal notes", "{notes}" }
			}

			if let Some(reasoning) = property.additional_reasoning.as_ref() {
				Note { label: "Reasoning", "{reasoning}" }
			}
		}
	}
}

/// A label/value row. Hairline-separated from the previous row via a top border
/// (first row drops it), matching the kit's deal-terms rhythm.
#[component]
fn Kv(label: String, #[props(default)] value_class: String, children: Element) -> Element {
	rsx! {
		div { class: "flex items-center justify-between gap-4 border-t border-border py-2.5 first:border-t-0",
			span { class: "text-sm text-muted-foreground", "{label}" }
			span { class: "text-right text-sm font-semibold {value_class}", {children} }
		}
	}
}

/// Developer row. Resolves the developer's note server-side and, when present,
/// surfaces it on hover over the name.
#[component]
fn DeveloperKv(name: String) -> Element {
	let lookup = name.clone();
	let dev = use_resource(move || {
		let lookup = lookup.clone();
		async move { crate::api::get_developer(lookup).await.ok().flatten() }
	});
	let note = dev.read().as_ref().and_then(|o| o.as_ref()).map(|d| d.note.clone()).filter(|n| !n.trim().is_empty());

	rsx! {
		match note {
			Some(note) => rsx! {
				Kv { label: "Developer",
					Tooltip {
						TooltipTrigger { class: "cursor-help text-sm font-semibold underline decoration-dotted underline-offset-4", "{name}" }
						TooltipContent { class: "max-w-xs text-left text-xs font-normal normal-case", "{note}" }
					}
				}
			},
			None => rsx! { Kv { label: "Developer", "{name}" } },
		}
	}
}

/// A free-text block (terms, notes, reasoning) under an eyebrow label.
#[component]
fn Note(label: String, children: Element) -> Element {
	rsx! {
		div { class: "mt-2 flex flex-col gap-1.5 border-t border-border pt-3",
			span { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground", "{label}" }
			p { class: "text-sm leading-relaxed text-muted-foreground", {children} }
		}
	}
}
