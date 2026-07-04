//! The EV brand chrome (EV-invest/site_conductor#55): the shared `ev_lib::uikit`
//! Header/Footer wrapped with this app's nav — every link points back at the
//! conductor, since REA is a standalone surface on its own subdomain.

use dioxus::prelude::*;
use ev_lib::uikit::{Footer, FooterLink, FooterLinkGroup, Header, HeaderNavItem};

/// site_conductor's outline-Button recipe, copied verbatim so the portal CTA is
/// pixel-identical across surfaces (the kit leaves CTA styling to the app).
const CTA: &str = "inline-flex items-center justify-center rounded-md border border-main-accent-t1 bg-transparent px-4 py-2 font-mono-tech text-xs tracking-wider text-main-accent-t1 transition-all duration-300 hover:bg-main-accent-t1 hover:text-main-black";

/// The deployed revision: the flake-passed `REA_BUILD_REV` (hermetic builds have
/// no `.git`) falling back to build.rs's `git rev-parse` `GIT_HASH`.
pub fn build_rev() -> &'static str {
	option_env!("REA_BUILD_REV").filter(|s| !s.is_empty()).unwrap_or(env!("GIT_HASH"))
}
#[component]
pub fn BrandHeader() -> Element {
	let origin = conductor_origin();
	let nav = [
		("Portfolio", "/#portfolio"),
		("Research", "/#research"),
		("Team", "/team"),
		("Hiring", "/hiring"),
		("Contact", "/contact"),
	]
	.into_iter()
	.map(|(label, path)| HeaderNavItem {
		label: label.to_string(),
		href: format!("{origin}{path}"),
	})
	.collect::<Vec<_>>();
	let portal = format!("{origin}/cabinet");
	rsx! {
		Header {
			nav,
			home_href: format!("{origin}/"),
			cta: rsx! {
				a { href: portal.clone(), class: "{CTA} hidden sm:inline-flex", "Investor Portal" }
			},
			mobile_cta: rsx! {
				a { href: portal, class: "{CTA} w-full justify-center py-6", "Investor Portal" }
			},
		}
	}
}
#[component]
pub fn BrandFooter() -> Element {
	let origin = conductor_origin();
	let link = |label: &str, path: &str| FooterLink {
		label: label.to_string(),
		href: format!("{origin}{path}"),
	};
	let nav = vec![
		FooterLinkGroup {
			heading: "Company".to_string(),
			links: vec![link("Home", "/"), link("Team", "/team"), link("Hiring", "/hiring"), link("Contact", "/contact")],
		},
		FooterLinkGroup {
			heading: "Explore".to_string(),
			links: vec![
				link("Portfolio", "/#portfolio"),
				link("Investment Calculator", "/#calculator"),
				link("Research Articles", "/blogs"),
				link("Whitepaper", "/whitepaper"),
			],
		},
	];
	let rev = build_rev();
	rsx! {
		Footer {
			nav,
			version: format!("v{}·{rev}", env!("CARGO_PKG_VERSION")),
			commit_href: format!("https://github.com/EV-invest/real_estate_allocation/commit/{rev}"),
		}
	}
}
/// Absolute origin of site_conductor, the hub the chrome's links point back to.
/// REA serves from its own subdomain (`rea.evinvest.ltd`), so root-relative
/// hrefs would stay on rea.* — every chrome link must carry the conductor's
/// origin. Dev derives from the same flake-baked `PORT` (landing's dev page)
/// that [`config`](crate::config)'s `default_cors_origins` uses, with the same
/// `next dev` fallback and parse-failure policy. It can't live in `config.rs`:
/// config is native-only (v_utils io/xdg) while these hrefs render in the wasm
/// client — which has no runtime config, so a build-time value is the only kind
/// it could ever have.
fn conductor_origin() -> String {
	if cfg!(debug_assertions) {
		let port: u16 = match option_env!("PORT") {
			Some(p) => p.parse().expect("PORT (build-time env) must be a valid u16"),
			None => 58843,
		};
		format!("http://localhost:{port}")
	} else {
		"https://evinvest.ltd".to_string()
	}
}



