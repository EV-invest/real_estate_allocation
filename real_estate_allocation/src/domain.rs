use ev::architecture::{AggregateRoot, Entity, Id, Specification};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct PropertyTag;
pub struct FileTag;
pub type PropertyId = Id<PropertyTag>;
pub type FileId = Id<FileTag>;

use crate::error::DomainError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PropertyState {
	Purchased,
	Interesting,
	Purchasing,
}

impl PropertyState {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Purchased => "purchased",
			Self::Interesting => "interesting",
			Self::Purchasing => "purchasing",
		}
	}

	pub fn parse(raw: &str) -> Result<Self, DomainError> {
		match raw {
			"purchased" => Ok(Self::Purchased),
			"interesting" => Ok(Self::Interesting),
			"purchasing" => Ok(Self::Purchasing),
			other => Err(DomainError::Validation(format!("unknown property state: {other}"))),
		}
	}
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Coords {
	pub lat: f64,
	pub lng: f64,
}

/// Required link to the research backing a property. A value object so the one
/// real invariant — a non-empty http(s) URL — is enforced at the boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ResearchUrl(String);

impl ResearchUrl {
	pub fn parse(raw: String) -> Result<Self, DomainError> {
		let trimmed = raw.trim();
		if trimmed.is_empty() {
			return Err(DomainError::Validation("research url must not be empty".into()));
		}
		if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
			return Err(DomainError::Validation("research url must be http(s)".into()));
		}
		Ok(Self(trimmed.to_owned()))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Money(f64);

impl Money {
	pub fn parse(raw: f64) -> Result<Self, DomainError> {
		if raw < 0.0 || !raw.is_finite() {
			return Err(DomainError::Validation("money must be a non-negative finite number".into()));
		}
		Ok(Self(raw))
	}

	pub fn amount(self) -> f64 {
		self.0
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DealStructure {
	pub equity_pct: f64,
	pub debt_pct: f64,
	pub notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LoanRates {
	pub rate_pct: f64,
	pub term_years: u32,
	pub lender: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Property {
	pub id: PropertyId,
	pub coords: Coords,
	pub price: Money,
	pub state: PropertyState,
	pub research_url: ResearchUrl,
	pub terms: Option<String>,
	pub deal: Option<DealStructure>,
	pub loan: Option<LoanRates>,
	pub additional_reasoning: Option<String>,
	/// Mocked weekly estimate series, filled by `api::get_property` via a
	/// random walk. Never persisted.
	#[serde(default)]
	pub price_series: Vec<f64>,
}

impl Entity for Property {
	type Id = PropertyId;

	fn id(&self) -> PropertyId {
		self.id
	}
}

impl AggregateRoot for Property {
	const NAME: &'static str = "property";
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FileKind {
	Pic,
	PitchDeck,
	Document,
}

impl FileKind {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Pic => "pic",
			Self::PitchDeck => "pitch_deck",
			Self::Document => "document",
		}
	}

	pub fn parse(raw: &str) -> Result<Self, DomainError> {
		match raw {
			"pic" => Ok(Self::Pic),
			"pitch_deck" => Ok(Self::PitchDeck),
			"document" => Ok(Self::Document),
			other => Err(DomainError::Validation(format!("unknown file kind: {other}"))),
		}
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PropertyFile {
	pub id: FileId,
	pub property_id: PropertyId,
	pub kind: FileKind,
	pub filename: String,
	pub content_type: String,
}

/// Map / portfolio filter. The portfolio default is `InState(Purchased)`; richer
/// views compose via `.or`, e.g. `InState(Interesting).or(InState(Purchasing))`.
pub struct InState(pub PropertyState);

impl Specification<Property> for InState {
	fn holds(&self, candidate: &Property) -> bool {
		candidate.state == self.0
	}
}

// Free functions, not inherent impls: `Id` is a foreign type, so coherence
// forbids `impl PropertyId { .. }` here.
pub fn parse_property_id(raw: &str) -> Result<PropertyId, DomainError> {
	Uuid::parse_str(raw)
		.map(PropertyId::from_raw)
		.map_err(|e| DomainError::Validation(format!("invalid property id: {e}")))
}

pub fn parse_file_id(raw: &str) -> Result<FileId, DomainError> {
	Uuid::parse_str(raw).map(FileId::from_raw).map_err(|e| DomainError::Validation(format!("invalid file id: {e}")))
}
