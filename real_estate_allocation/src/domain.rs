use ev::architecture::{AggregateRoot, Entity, Id, Specification};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type PropertyId = Id<PropertyTag>;
pub type FileId = Id<FileTag>;

use crate::error::DomainError;

pub struct FileTag;

pub struct PropertyTag;

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

/// Build progress, orthogonal to `PropertyState` (which tracks *our* acquisition
/// lifecycle, not the asset's physical state).
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConstructionStatus {
	UnderConstruction,
	Completed,
}

impl ConstructionStatus {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::UnderConstruction => "under_construction",
			Self::Completed => "completed",
		}
	}

	pub fn parse(raw: &str) -> Result<Self, DomainError> {
		match raw {
			"under_construction" => Ok(Self::UnderConstruction),
			"completed" => Ok(Self::Completed),
			other => Err(DomainError::Validation(format!("unknown construction status: {other}"))),
		}
	}
}

/// A developer we know. Referenced by `Property::developer` (by `name`); the store
/// enforces that every non-null reference resolves to one of these.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Developer {
	pub name: String,
	//TODO: generalize this `note` into a reusable concept — an arbitrary (table,
	// key) → note side-table surfaced on hover, so any value (not just developers)
	// can carry one, and ideally the lookup tables themselves are defined/managed
	// through it. For now it lives only on Developer.
	pub note: String,
	/// The developer's own homepage. Per-property brochures live in documents, not
	/// here — this is the developer-level link only.
	pub page: Option<String>,
}

/// A Google Place ID — our canonical handle on a property's location. The map
/// resolves it to a pin via Google; name / address / coordinates are derived from
/// it at render time rather than stored by hand.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct GooglePlace(String);

impl GooglePlace {
	pub fn parse(raw: String) -> Result<Self, DomainError> {
		let trimmed = raw.trim();
		if trimmed.is_empty() {
			return Err(DomainError::Validation("google place id must not be empty".into()));
		}
		Ok(Self(trimmed.to_owned()))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
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

impl std::fmt::Display for Money {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let a = self.0;
		if a >= 1_000_000.0 {
			write!(f, "${:.2}M", a / 1_000_000.0)
		} else if a >= 1_000.0 {
			write!(f, "${:.0}K", a / 1_000.0)
		} else {
			write!(f, "${a:.0}")
		}
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
	pub name: String,
	pub place: GooglePlace,
	/// `None` until we have a real number — rendered as a `?` in the warn colour
	/// rather than a fabricated figure.
	pub price: Option<Money>,
	pub state: PropertyState,
	pub construction: ConstructionStatus,
	/// Developer name; must resolve to a row in the developers table when set.
	pub developer: Option<String>,
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
