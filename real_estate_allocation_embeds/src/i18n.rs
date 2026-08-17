//! Locale wiring for the embed bundle.
//!
//! Loading and policy come from [`real_estate_allocation_core::i18n`], shared
//! with the dashboard. The catalogue does not: this surface's copy is marketing
//! and the dashboard's is operational, and one vocabulary cannot serve both
//! audiences.
//!
//! **Only the labels are translated.** The property descriptions and the
//! market-thesis prose in [`view`](crate::view) stay as written — they are
//! authored copy, the same class as the dataset in the dashboard's `store`, and
//! the same call the public site made for its long-form sections. A translated
//! frame around English prose is the worse of the two failures.

use dioxus::prelude::*;
use ev_lib::i18n::{DEFAULT_LOCALE, Locale, Translator};
use real_estate_allocation_core::i18n::Catalogues;

/// The cookie the conductor sets once it has negotiated a reader's language.
///
/// The bundle is a cross-origin script, but it executes in the **host page's**
/// document — so this reads the conductor's cookie, which is exactly the one the
/// shell wrote. No separate handshake, and no second negotiation that could
/// disagree with the header rendered above the widget.
const LOCALE_COOKIE: &str = "ev_locale";

pub const CATALOGUES: Catalogues = Catalogues {
	en: include_str!("../messages/en/common.json"),
	ru: include_str!("../messages/ru/common.json"),
	vi: include_str!("../messages/vi/common.json"),
	fr: include_str!("../messages/fr/common.json"),
	de: include_str!("../messages/de/common.json"),
};

#[cfg(target_arch = "wasm32")]
fn detect() -> Locale {
	use wasm_bindgen::JsCast as _;

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

/// The static snapshot renders natively, with no document to read a cookie from.
///
/// So `portfolio.html` is English — and that is the right answer rather than a
/// gap: the snapshot is what the host shows *until the live bundle upgrades it*,
/// and the canonical locale is the honest thing to bake into a file that has to
/// serve every reader at once.
#[cfg(not(target_arch = "wasm32"))]
fn detect() -> Locale {
	let _ = LOCALE_COOKIE;
	DEFAULT_LOCALE
}

/// Shared from the widget root so every cell renders in one language.
pub type I18n = Signal<Translator>;

/// Install the translator. Call once, at the widget root.
pub fn use_provide_i18n() -> I18n {
	use_context_provider(|| {
		let locale = detect();
		Signal::new(Translator::new(CATALOGUES.resolve(locale).messages, locale))
	})
}

/// The translator for the current locale.
///
/// Degrades to the canonical locale when the provider is absent — a cell
/// rendered on its own must not crash over a label. (Still needs a Dioxus
/// runtime; this only drops the *provider* as a hard requirement.)
pub fn use_t() -> Translator {
	match try_consume_context::<I18n>() {
		Some(signal) => signal(),
		None => Translator::new(CATALOGUES.resolve(DEFAULT_LOCALE).messages, DEFAULT_LOCALE),
	}
}

/// Renders a label whose accent word is marked `*like this*`.
///
/// The heading sets one word in italic serif — "Premium Asset *Portfolio*".
/// Splitting that into two keys would hard-code English word order: Russian and
/// German both put the adjective elsewhere in the phrase, and a translator
/// handed `{lead} <em>{accent}</em>` cannot move it. One key with an inline
/// marker lets the sentence be rearranged and keeps the typography.
#[component]
pub fn Accented(text: String, class: String) -> Element {
	rsx! {
		// Odd segments are the marked ones: "a *b* c" -> ["a ", "b", " c"].
		for (i , part) in text.split('*').enumerate() {
			if i % 2 == 1 {
				span { class: "{class}", "{part}" }
			} else {
				"{part}"
			}
		}
	}
}
