//! Locale wiring for the dashboard.
//!
//! The catalogue loading and the policy live in
//! [`real_estate_allocation_core::i18n`], shared with the embed bundle. What is
//! here is only what differs per surface: this crate's own `messages/`, and the
//! Dioxus context that hands one translator to every panel.
//!
//! The property dataset in [`store`](crate::store) is content authored per
//! building, not chrome, so it does not live here. Its two prose fields — terms
//! and reasoning — are translated per record in [`translations`](crate::translations),
//! the same route publication cards took. Project names and developers are
//! proper nouns and stay as written.

use dioxus::prelude::*;
use ev_lib::i18n::{DEFAULT_LOCALE, Locale, Translator};
use real_estate_allocation_core::i18n::Catalogues;
pub use real_estate_allocation_core::i18n::status_key;

/// The cookie the conductor sets once it has negotiated a reader's language.
/// A zone never negotiates for itself — the shell owns that decision, and two
/// negotiators disagreeing is how a reader gets a Russian header over a French
/// page.
const LOCALE_COOKIE: &str = "ev_locale";

/// This surface's copy: operational, not marketing. The embed bundle keeps its
/// own for the opposite reason.
pub const CATALOGUES: Catalogues = Catalogues {
	en: include_str!("../messages/en/common.json"),
	ru: include_str!("../messages/ru/common.json"),
	vi: include_str!("../messages/vi/common.json"),
	fr: include_str!("../messages/fr/common.json"),
	de: include_str!("../messages/de/common.json"),
};

/// The reader's locale, from the cookie the conductor set.
///
/// Deliberately does **not** fall back to `navigator.language`. The zone is
/// always mounted inside the conductor's shell, which has already negotiated and
/// written the cookie; guessing again here could disagree with the header
/// rendered directly above this view. No cookie means the shell chose the
/// default, which is exactly what an unrecognised value means too.
#[cfg(target_arch = "wasm32")]
fn detect() -> Locale {
	let Some(cookies) = web_sys::window()
		.and_then(|w| w.document())
		.and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
		.and_then(|d| d.cookie().ok())
	else {
		return DEFAULT_LOCALE;
	};
	cookies
		.split(';')
		.filter_map(|pair| pair.split_once('='))
		.find(|(name, _)| name.trim() == LOCALE_COOKIE)
		.and_then(|(_, value)| Locale::parse(value.trim()))
		.unwrap_or(DEFAULT_LOCALE)
}

/// Server-side render: no document, so the default. The client re-resolves on
/// mount, which is the same path every other cookie-backed value in this app
/// takes.
#[cfg(not(target_arch = "wasm32"))]
fn detect() -> Locale {
	let _ = LOCALE_COOKIE;
	DEFAULT_LOCALE
}

/// Shared from the root so every panel renders in one language. A panel that
/// built its own translator would be a second place the locale could be wrong.
pub type I18n = Signal<Translator>;

/// Install the translator. Call once, at the app root.
pub fn use_provide_i18n() -> I18n {
	use_context_provider(|| {
		let locale = detect();
		Signal::new(Translator::new(CATALOGUES.resolve(locale).messages, locale))
	})
}

/// The translator for the current locale.
///
/// ```ignore
/// let t = use_t();
/// rsx! { span { "{t(\"panel.map\")}" } }
/// ```
pub fn use_t() -> Translator {
	// `use_context` panics when the provider is absent, which would turn a panel
	// rendered outside the app root — a prerender of one component, a story —
	// into a crash over a *label*. Degrade to the canonical locale instead: the
	// same choice this module makes everywhere else.
	//
	// This still requires a Dioxus runtime; it only removes the *provider* as a
	// hard requirement, which is why there is no unit test for it (a bare test
	// has no runtime at all and panics before reaching this line).
	match try_consume_context::<I18n>() {
		Some(signal) => signal(),
		None => Translator::new(CATALOGUES.resolve(DEFAULT_LOCALE).messages, DEFAULT_LOCALE),
	}
}

#[cfg(test)]
mod tests {
	use ev_lib::i18n::LOCALES;

	use super::*;

	#[test]
	fn every_catalogue_carries_every_english_key() {
		let expected = CATALOGUES.key_count();
		assert!(expected > 0, "the English catalogue is the key set — it cannot be empty");
		for locale in LOCALES {
			assert_eq!(CATALOGUES.resolve(locale).messages.len(), expected, "{locale} does not cover every key English defines");
		}
	}

	#[test]
	fn no_translation_has_drifted_from_its_english_source() {
		let (ok, report) = CATALOGUES.audit();
		assert!(ok, "\n{report}\n");
	}

	#[test]
	fn a_translated_panel_title_actually_resolves() {
		let t = Translator::new(CATALOGUES.resolve(Locale::Ru).messages, Locale::Ru);
		assert_eq!(t.t("panel.map"), "Карта");
		assert_eq!(t.t("status.purchased"), "Куплено");
	}

	/// Placeholder names are part of what the policy checks, so this pins that
	/// the argument survives translation in every locale.
	#[test]
	fn the_apartment_label_interpolates_in_every_locale() {
		for locale in LOCALES {
			let t = Translator::new(CATALOGUES.resolve(locale).messages, locale);
			let out = t.tv("header.apt", &[("n".to_owned(), 12.into())].into_iter().collect());
			assert!(out.contains("12"), "{locale} dropped the apartment number: {out}");
		}
	}
}
