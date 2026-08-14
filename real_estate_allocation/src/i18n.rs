//! Locale wiring for the dashboard.
//!
//! The catalogues under `messages/` are **the same on-disk format the public
//! site uses** — `en/common.json` is flat `key → text`, every other locale is
//! `key → {en, t}` carrying the English it was translated from. That is what
//! lets `ev_lib::i18n::policy` apply the same rules here as
//! `npm run i18n:check` applies there, and it is why a catalogue can move
//! between the two without conversion.
//!
//! Baked in with `include_str!` rather than fetched: this is a wasm SPA, so a
//! runtime fetch would mean the first paint renders untranslated and then
//! reflows, and a failed fetch would mean no copy at all. Five small JSON files
//! cost less than that flicker.
//!
//! What is **not** translated: the property dataset in [`store`](crate::store)
//! — project names, developers, terms, reasoning. That is content authored per
//! building, not chrome, and it takes the same route publications did: it stays
//! in the language it was written in until there is a per-record translation to
//! serve. Translating the frame around English prose would be the worse of the
//! two failures.

use std::collections::BTreeMap;

use dioxus::prelude::*;
use ev_lib::i18n::{
	DEFAULT_LOCALE, LOCALES, Locale, Messages, Translator,
	policy::{ResolvedCatalogue, TranslatedCatalogue, TranslatedEntry, resolve_catalogue},
};
use serde::Deserialize;

use crate::domain::ApartmentStatus;

/// The cookie the conductor sets once it has negotiated a reader's language.
/// A zone never negotiates for itself — the shell owns that decision, and two
/// negotiators disagreeing is how a reader gets a Russian header over a French
/// page.
const LOCALE_COOKIE: &str = "ev_locale";

const EN: &str = include_str!("../messages/en/common.json");
const RU: &str = include_str!("../messages/ru/common.json");
const VI: &str = include_str!("../messages/vi/common.json");
const FR: &str = include_str!("../messages/fr/common.json");
const DE: &str = include_str!("../messages/de/common.json");

/// `{"en": "...", "t": "..."}` — the wire shape. `ev_lib` stays dep-free and so
/// derives no serde; converting here is a four-line cost for keeping the
/// library parseable by anything.
#[derive(Deserialize)]
struct RawEntry {
	en: String,
	t: String,
}

fn source() -> Messages {
	serde_json::from_str(EN).expect("messages/en/common.json is malformed")
}

fn translated(raw: &str) -> TranslatedCatalogue {
	let parsed: BTreeMap<String, RawEntry> = serde_json::from_str(raw).expect("a locale catalogue is malformed");
	parsed.into_iter().map(|(k, v)| (k, TranslatedEntry { en: v.en, t: v.t })).collect()
}

fn raw_for(locale: Locale) -> Option<&'static str> {
	match locale {
		Locale::En => None,
		Locale::Ru => Some(RU),
		Locale::Vi => Some(VI),
		Locale::Fr => Some(FR),
		Locale::De => Some(DE),
	}
}

/// Apply the translation policy to one locale.
///
/// Rule 1.2 in practice: an entry whose English source has moved is refused and
/// the English is served instead, so a stale translation cannot quietly
/// contradict the dashboard. Nothing here can fail into a blank panel — every
/// key English defines is present in the result.
pub fn resolve(locale: Locale) -> ResolvedCatalogue {
	let en = source();
	match raw_for(locale) {
		None => resolve_catalogue(DEFAULT_LOCALE, &en, &TranslatedCatalogue::new()),
		Some(raw) => resolve_catalogue(locale, &en, &translated(raw)),
	}
}

/// Every non-English catalogue, resolved — what the drift test reports on.
pub fn resolve_all() -> Vec<ResolvedCatalogue> {
	LOCALES.into_iter().filter(|l| *l != DEFAULT_LOCALE).map(resolve).collect()
}

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

/// The catalogue key for an apartment's portfolio state.
///
/// One mapping, used by the header, the details panel and anything else that
/// shows a state. Two panels each holding their own copy is how a building ends
/// up "Purchased" in one place and something else in another — which is exactly
/// what was happening here before.
pub fn status_key(status: ApartmentStatus) -> &'static str {
	match status {
		ApartmentStatus::Available => "status.available",
		ApartmentStatus::Sold => "status.sold",
		ApartmentStatus::Purchasing => "status.purchasing",
		ApartmentStatus::Purchased(_) => "status.purchased",
		ApartmentStatus::Interesting => "status.interesting",
	}
}

/// Shared from the root so every panel renders in one language. A panel that
/// built its own translator would be a second place the locale could be wrong.
pub type I18n = Signal<Translator>;

/// Install the translator. Call once, at the app root.
pub fn use_provide_i18n() -> I18n {
	use_context_provider(|| {
		let locale = detect();
		Signal::new(Translator::new(resolve(locale).messages, locale))
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
		None => Translator::new(resolve(DEFAULT_LOCALE).messages, DEFAULT_LOCALE),
	}
}

#[cfg(test)]
mod tests {
	use ev_lib::i18n::policy::audit;

	use super::*;

	#[test]
	fn every_catalogue_parses_and_carries_every_english_key() {
		let en = source();
		assert!(!en.is_empty(), "the English catalogue is the key set — it cannot be empty");
		for locale in LOCALES {
			let resolved = resolve(locale);
			assert_eq!(resolved.messages.len(), en.len(), "{locale} does not cover every key English defines");
		}
	}

	/// The drift gate, and the mirror of `npm run i18n:check` on the site.
	///
	/// The runtime already degrades safely — rule 1.2 serves English and the
	/// panel is fine. That safety is exactly why drift needs a second, noisy
	/// channel: a silent fallback looks identical to a zone that was never
	/// translated, so without this a locale can rot to zero coverage unnoticed.
	#[test]
	fn no_translation_has_drifted_from_its_english_source() {
		let (ok, report) = audit(&resolve_all(), 1.0);
		assert!(ok, "\n{report}\n");
	}

	#[test]
	fn a_translated_panel_title_actually_resolves() {
		let t = Translator::new(resolve(Locale::Ru).messages, Locale::Ru);
		assert_eq!(t.t("panel.map"), "Карта");
		assert_eq!(t.t("status.purchased"), "Куплено");
	}

	/// The interpolated key in the breadcrumb. Placeholder names are part of what
	/// the policy checks, so this pins that the argument survives translation.
	#[test]
	fn the_apartment_label_interpolates_in_every_locale() {
		for locale in LOCALES {
			let t = Translator::new(resolve(locale).messages, locale);
			let out = t.tv("header.apt", &[("n".to_owned(), 12.into())].into_iter().collect());
			assert!(out.contains("12"), "{locale} dropped the apartment number: {out}");
		}
	}
}
