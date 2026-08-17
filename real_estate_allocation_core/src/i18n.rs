//! Catalogue loading and policy, shared by every REA surface.
//!
//! Deliberately **no Dioxus** — this crate is the pure layer, and the hook wiring
//! (a context provider, a `use_t`) differs per surface anyway. What is genuinely
//! common is the part worth having exactly once: parsing the on-disk shape,
//! applying `ev_lib::i18n::policy`, and reporting drift.
//!
//! Each surface owns its own `messages/` and hands them over as
//! [`Catalogues`]. The dashboard's copy is operational — panel titles, portfolio
//! states — and the embed's is marketing; sharing one catalogue between them
//! would force one vocabulary onto two audiences. Sharing the *mechanism* costs
//! nothing and keeps a single definition of what "translated" means.
//!
//! The files are the same format the public site uses: `en/common.json` flat
//! `key → text`, every other locale `key → {en, t}` carrying the English it was
//! translated from. That is what lets one policy serve both halves of the stack.

use std::collections::BTreeMap;

use ev_lib::i18n::{
	DEFAULT_LOCALE, LOCALES, Locale, Messages,
	policy::{ResolvedCatalogue, TranslatedCatalogue, TranslatedEntry, audit, resolve_catalogue},
};
use serde::Deserialize;

use crate::domain::ApartmentStatus;

/// `{"en": "...", "t": "..."}` — the wire shape.
///
/// `ev_lib` is dependency-free and so derives no serde; converting here is a
/// few lines for keeping the library parseable by anything.
#[derive(Deserialize)]
struct RawEntry {
	en: String,
	t: String,
}

/// One surface's five catalogues, baked in at compile time.
///
/// `include_str!` rather than a runtime fetch: these ship inside a wasm bundle,
/// where fetching would mean the first paint renders untranslated and then
/// reflows, and a failed fetch would mean no copy at all.
#[derive(Debug, Clone, Copy)]
pub struct Catalogues {
	pub en: &'static str,
	pub ru: &'static str,
	pub vi: &'static str,
	pub fr: &'static str,
	pub de: &'static str,
}

impl Catalogues {
	fn source(&self) -> Messages {
		serde_json::from_str(self.en).expect("the English catalogue is malformed")
	}

	fn raw_for(&self, locale: Locale) -> Option<&'static str> {
		match locale {
			Locale::En => None,
			Locale::Ru => Some(self.ru),
			Locale::Vi => Some(self.vi),
			Locale::Fr => Some(self.fr),
			Locale::De => Some(self.de),
		}
	}

	/// Apply the translation policy to one locale.
	///
	/// Rule 1.2 in practice: an entry whose English source has moved is refused
	/// and the English served instead, so a stale translation cannot quietly
	/// contradict the page. This cannot fail into a blank label — every key
	/// English defines is present in the result.
	pub fn resolve(&self, locale: Locale) -> ResolvedCatalogue {
		let en = self.source();
		match self.raw_for(locale) {
			None => resolve_catalogue(DEFAULT_LOCALE, &en, &TranslatedCatalogue::new()),
			Some(raw) => {
				let parsed: BTreeMap<String, RawEntry> = serde_json::from_str(raw).unwrap_or_else(|e| panic!("the {locale} catalogue is malformed: {e}"));
				let translated: TranslatedCatalogue = parsed.into_iter().map(|(k, v)| (k, TranslatedEntry { en: v.en, t: v.t })).collect();
				resolve_catalogue(locale, &en, &translated)
			}
		}
	}

	/// Every non-English catalogue, resolved — what the drift check reports on.
	pub fn resolve_all(&self) -> Vec<ResolvedCatalogue> {
		LOCALES.into_iter().filter(|l| *l != DEFAULT_LOCALE).map(|l| self.resolve(l)).collect()
	}

	/// The drift gate, and the in-repo mirror of the site's `npm run i18n:check`.
	///
	/// The runtime already degrades safely — rule 1.2 serves English and the page
	/// is fine. That safety is exactly why drift needs a second, noisy channel: a
	/// silent fallback looks identical to a surface that was never translated, so
	/// without this a locale can rot to zero coverage unnoticed.
	pub fn audit(&self) -> (bool, String) {
		audit(&self.resolve_all(), 1.0)
	}

	/// Number of keys English defines — the size every locale must match.
	pub fn key_count(&self) -> usize {
		self.source().len()
	}
}

/// The catalogue key for an apartment's portfolio state.
///
/// One mapping for the whole repo. Two surfaces each holding their own copy is
/// how a building ends up "Purchased" in one place and something else in
/// another — which is what the dashboard's header and details panels were
/// already doing before this existed.
pub fn status_key(status: ApartmentStatus) -> &'static str {
	match status {
		ApartmentStatus::Available => "status.available",
		ApartmentStatus::Sold => "status.sold",
		ApartmentStatus::Purchasing => "status.purchasing",
		ApartmentStatus::Purchased(_) => "status.purchased",
		ApartmentStatus::Interesting => "status.interesting",
	}
}
