//! Catalogue gates for the embed bundle.
//!
//! `src/lib.rs` is `#![cfg(target_arch = "wasm32")]`, so anything behind
//! `#[cfg(test)]` in the crate never compiles on the host and never runs in CI.
//! Pulling the module in by path — the same trick `examples/portfolio_snapshot.rs`
//! uses for `view` — is what makes these actually execute.

#[path = "../src/i18n.rs"]
mod i18n;

use ev_lib::i18n::{LOCALES, Translator};

use crate::i18n::CATALOGUES;

#[test]
fn every_catalogue_carries_every_english_key() {
	let expected = CATALOGUES.key_count();
	assert!(expected > 0, "the English catalogue is the key set — it cannot be empty");
	for locale in LOCALES {
		assert_eq!(CATALOGUES.resolve(locale).messages.len(), expected, "{locale} does not cover every key English defines");
	}
}

/// The drift gate — the in-repo mirror of the site's `npm run i18n:check`.
///
/// The runtime already degrades safely: rule 1.2 serves English and the widget
/// is fine. That safety is why drift needs a second, noisy channel — a silent
/// fallback looks identical to a surface nobody ever translated.
#[test]
fn no_translation_has_drifted_from_its_english_source() {
	let (ok, report) = CATALOGUES.audit();
	assert!(ok, "\n{report}\n");
}

/// The accent marker has to survive translation or the heading loses its italic
/// word. The policy does not check for it — it is our convention, not ICU — so
/// it is pinned here.
#[test]
fn every_locale_keeps_the_accent_marker_in_the_title() {
	for locale in LOCALES {
		let t = Translator::new(CATALOGUES.resolve(locale).messages, locale);
		let title = t.t("embeds.title");
		assert_eq!(title.matches('*').count(), 2, "{locale} lost the accent marker: {title}");
	}
}
