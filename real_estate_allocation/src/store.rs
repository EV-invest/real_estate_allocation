use std::path::{Path, PathBuf};

use async_trait::async_trait;
use ev::architecture::{Reader, Repository, Specification};
use sqlx::{FromRow, Row as _, SqlitePool, sqlite::SqlitePoolOptions};

use crate::{
	domain::{Coords, DealStructure, FileId, FileKind, LoanRates, Money, Property, PropertyFile, PropertyId, PropertyState, ResearchUrl},
	error::DomainError,
};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS properties (
	id TEXT PRIMARY KEY,
	lat REAL NOT NULL,
	lng REAL NOT NULL,
	price REAL NOT NULL,
	state TEXT NOT NULL,
	research_url TEXT NOT NULL,
	terms TEXT,
	deal_json TEXT,
	loan_json TEXT,
	additional_reasoning TEXT
);
CREATE TABLE IF NOT EXISTS property_files (
	id TEXT PRIMARY KEY,
	property_id TEXT NOT NULL REFERENCES properties(id),
	kind TEXT NOT NULL,
	filename TEXT NOT NULL,
	content_type TEXT NOT NULL
);
";

const SAMPLE_PIC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360" viewBox="0 0 640 360"><rect width="640" height="360" fill="#0c1626"/><rect x="40" y="180" width="120" height="140" fill="#001e4e"/><rect x="200" y="120" width="140" height="200" fill="#081020"/><rect x="380" y="60" width="160" height="260" fill="#001e4e"/><text x="320" y="340" fill="#e6e1d3" font-family="serif" font-size="20" text-anchor="middle">Sample Property</text></svg>"##;
/// Leaf port over the `ev` repository markers. No `UnitOfWork`: every write here
/// is a single row, so a transaction boundary would buy nothing.
#[async_trait]
pub trait PropertyRepository: Repository<Aggregate = Property> + Reader<Aggregate = Property> {
	async fn list(&self, spec: Option<&(dyn Specification<Property> + Sync)>) -> Result<Vec<Property>, DomainError>;
	async fn get(&self, id: PropertyId) -> Result<Option<Property>, DomainError>;
	async fn add_file(&self, f: PropertyFile) -> Result<(), DomainError>;
	async fn list_files(&self, id: PropertyId) -> Result<Vec<PropertyFile>, DomainError>;
}

#[derive(Clone)]
pub struct SqliteStore {
	pool: SqlitePool,
	data_dir: PathBuf,
}

impl SqliteStore {
	pub async fn open(db_path: &str, data_dir: PathBuf) -> Result<Self, DomainError> {
		if let Some(parent) = Path::new(db_path).parent() {
			std::fs::create_dir_all(parent).map_err(|e| DomainError::Repository(format!("create db dir: {e}")))?;
		}
		std::fs::create_dir_all(&data_dir).map_err(|e| DomainError::Repository(format!("create data dir: {e}")))?;

		let pool = SqlitePoolOptions::new().connect(&format!("sqlite://{db_path}?mode=rwc")).await.map_err(map_sqlx_error)?;
		sqlx::query(SCHEMA).execute(&pool).await.map_err(map_sqlx_error)?;
		Ok(Self { pool, data_dir })
	}

	/// `./data/properties/<property_id>/<file_id>__<filename>`.
	pub fn file_path(&self, property_id: PropertyId, file_id: FileId, filename: &str) -> PathBuf {
		self.data_dir.join(property_id.raw().to_string()).join(format!("{}__{filename}", file_id.raw()))
	}
}

/// Insert a handful of sample properties across all three states if the table is
/// empty. Idempotent: a non-empty DB is left untouched.
pub async fn seed(store: &SqliteStore) -> Result<(), DomainError> {
	let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM properties").fetch_one(&store.pool).await.map_err(map_sqlx_error)?.get("n");
	if count > 0 {
		return Ok(());
	}

	struct Seed {
		coords: Coords,
		price: f64,
		state: PropertyState,
		research_url: &'static str,
		terms: Option<&'static str>,
		deal: Option<DealStructure>,
		loan: Option<LoanRates>,
		reasoning: Option<&'static str>,
	}

	let seeds = [
		Seed {
			coords: Coords { lat: 40.7580, lng: -73.9855 },
			price: 12_500_000.0,
			state: PropertyState::Purchased,
			research_url: "https://example.com/research/times-square",
			terms: Some("All-cash close, 30-day diligence."),
			deal: Some(DealStructure {
				equity_pct: 60.0,
				debt_pct: 40.0,
				notes: Some("JV with local operator".into()),
			}),
			loan: Some(LoanRates {
				rate_pct: 5.4,
				term_years: 10,
				lender: "Apollo".into(),
			}),
			reasoning: Some("Flagship retail corridor; durable foot traffic."),
		},
		Seed {
			coords: Coords { lat: 51.5074, lng: -0.1278 },
			price: 8_900_000.0,
			state: PropertyState::Purchased,
			research_url: "https://example.com/research/london-city",
			terms: None,
			deal: None,
			loan: Some(LoanRates {
				rate_pct: 4.9,
				term_years: 7,
				lender: "Lloyds".into(),
			}),
			reasoning: None,
		},
		Seed {
			coords: Coords { lat: 48.8566, lng: 2.3522 },
			price: 6_400_000.0,
			state: PropertyState::Interesting,
			research_url: "https://example.com/research/paris-marais",
			terms: Some("Seller financing on offer."),
			deal: None,
			loan: None,
			reasoning: Some("Mixed-use upside if rezoned."),
		},
		Seed {
			coords: Coords { lat: 35.6762, lng: 139.6503 },
			price: 5_100_000.0,
			state: PropertyState::Interesting,
			research_url: "https://example.com/research/tokyo-shibuya",
			terms: None,
			deal: None,
			loan: None,
			reasoning: None,
		},
		Seed {
			coords: Coords { lat: 25.7617, lng: -80.1918 },
			price: 7_750_000.0,
			state: PropertyState::Purchasing,
			research_url: "https://example.com/research/miami-brickell",
			terms: Some("Under LOI, exclusivity through Q3."),
			deal: Some(DealStructure {
				equity_pct: 75.0,
				debt_pct: 25.0,
				notes: None,
			}),
			loan: Some(LoanRates {
				rate_pct: 6.1,
				term_years: 5,
				lender: "Blackstone".into(),
			}),
			reasoning: Some("Tax-advantaged inflow tailwind."),
		},
		Seed {
			coords: Coords { lat: 1.3521, lng: 103.8198 },
			price: 9_300_000.0,
			state: PropertyState::Purchasing,
			research_url: "https://example.com/research/singapore-cbd",
			terms: None,
			deal: None,
			loan: None,
			reasoning: Some("Gateway-city scarcity premium."),
		},
	];

	let mut first_id = None;
	for s in seeds {
		let id = PropertyId::new();
		if first_id.is_none() {
			first_id = Some(id);
		}
		let deal_json = s.deal.as_ref().map(|d| serde_json::to_string(d).expect("DealStructure serializes"));
		let loan_json = s.loan.as_ref().map(|l| serde_json::to_string(l).expect("LoanRates serializes"));
		sqlx::query("INSERT INTO properties (id, lat, lng, price, state, research_url, terms, deal_json, loan_json, additional_reasoning) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
			.bind(id.raw().to_string())
			.bind(s.coords.lat)
			.bind(s.coords.lng)
			.bind(s.price)
			.bind(s.state.as_str())
			.bind(s.research_url)
			.bind(s.terms)
			.bind(deal_json)
			.bind(loan_json)
			.bind(s.reasoning)
			.execute(&store.pool)
			.await
			.map_err(map_sqlx_error)?;
	}

	// Drop a sample pic to disk + metadata, attached to the first seeded property.
	if let Some(pid) = first_id {
		let fid = FileId::new();
		let filename = "sample.svg";
		let bytes = SAMPLE_PIC.as_bytes();
		let path = store.file_path(pid, fid, filename);
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent).map_err(|e| DomainError::Repository(format!("create file dir: {e}")))?;
		}
		std::fs::write(&path, bytes).map_err(|e| DomainError::Repository(format!("write sample pic: {e}")))?;
		store
			.add_file(PropertyFile {
				id: fid,
				property_id: pid,
				kind: FileKind::Pic,
				filename: filename.into(),
				content_type: "image/svg+xml".into(),
			})
			.await?;
	}

	Ok(())
}
/// Row mirroring the `properties` table. Private so the domain model stays free
/// of persistence derives.
#[derive(FromRow)]
struct PropertyRow {
	id: String,
	lat: f64,
	lng: f64,
	price: f64,
	state: String,
	research_url: String,
	terms: Option<String>,
	deal_json: Option<String>,
	loan_json: Option<String>,
	additional_reasoning: Option<String>,
}

impl TryFrom<PropertyRow> for Property {
	type Error = DomainError;

	/// Stored values were validated on the way in, so a parse failure here means
	/// the row is corrupt — a repository error, never a client-facing validation.
	fn try_from(row: PropertyRow) -> Result<Self, Self::Error> {
		let id = crate::domain::parse_property_id(&row.id).map_err(corrupt_row)?;
		let deal = row
			.deal_json
			.map(|j| serde_json::from_str::<DealStructure>(&j))
			.transpose()
			.map_err(|e| corrupt_row(DomainError::Repository(e.to_string())))?;
		let loan = row
			.loan_json
			.map(|j| serde_json::from_str::<LoanRates>(&j))
			.transpose()
			.map_err(|e| corrupt_row(DomainError::Repository(e.to_string())))?;
		Ok(Self {
			id,
			coords: Coords { lat: row.lat, lng: row.lng },
			price: Money::parse(row.price).map_err(corrupt_row)?,
			state: PropertyState::parse(&row.state).map_err(corrupt_row)?,
			research_url: ResearchUrl::parse(row.research_url).map_err(corrupt_row)?,
			terms: row.terms,
			deal,
			loan,
			additional_reasoning: row.additional_reasoning,
			price_series: Vec::new(),
		})
	}
}

#[derive(FromRow)]
struct FileRow {
	id: String,
	property_id: String,
	kind: String,
	filename: String,
	content_type: String,
}

impl TryFrom<FileRow> for PropertyFile {
	type Error = DomainError;

	fn try_from(row: FileRow) -> Result<Self, Self::Error> {
		Ok(Self {
			id: crate::domain::parse_file_id(&row.id).map_err(corrupt_row)?,
			property_id: crate::domain::parse_property_id(&row.property_id).map_err(corrupt_row)?,
			kind: FileKind::parse(&row.kind).map_err(corrupt_row)?,
			filename: row.filename,
			content_type: row.content_type,
		})
	}
}

impl Repository for SqliteStore {
	type Aggregate = Property;
}

impl Reader for SqliteStore {
	type Aggregate = Property;
}

#[async_trait]
impl PropertyRepository for SqliteStore {
	/// Rows are loaded then filtered in Rust via `spec.holds`. The spec engine is
	/// in-memory by design; SQL pushdown is explicitly descoped.
	async fn list(&self, spec: Option<&(dyn Specification<Property> + Sync)>) -> Result<Vec<Property>, DomainError> {
		let rows = sqlx::query_as::<_, PropertyRow>("SELECT id, lat, lng, price, state, research_url, terms, deal_json, loan_json, additional_reasoning FROM properties")
			.fetch_all(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		let mut out = Vec::new();
		for row in rows {
			let p = Property::try_from(row)?;
			if spec.map(|s| Specification::holds(s, &p)).unwrap_or(true) {
				out.push(p);
			}
		}
		Ok(out)
	}

	async fn get(&self, id: PropertyId) -> Result<Option<Property>, DomainError> {
		let row = sqlx::query_as::<_, PropertyRow>("SELECT id, lat, lng, price, state, research_url, terms, deal_json, loan_json, additional_reasoning FROM properties WHERE id = ?")
			.bind(id.raw().to_string())
			.fetch_optional(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		row.map(Property::try_from).transpose()
	}

	async fn add_file(&self, f: PropertyFile) -> Result<(), DomainError> {
		sqlx::query("INSERT INTO property_files (id, property_id, kind, filename, content_type) VALUES (?, ?, ?, ?, ?)")
			.bind(f.id.raw().to_string())
			.bind(f.property_id.raw().to_string())
			.bind(f.kind.as_str())
			.bind(&f.filename)
			.bind(&f.content_type)
			.execute(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn list_files(&self, id: PropertyId) -> Result<Vec<PropertyFile>, DomainError> {
		let rows = sqlx::query_as::<_, FileRow>("SELECT id, property_id, kind, filename, content_type FROM property_files WHERE property_id = ?")
			.bind(id.raw().to_string())
			.fetch_all(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		rows.into_iter().map(PropertyFile::try_from).collect()
	}
}

/// A persisted row failed domain re-validation: the database holds bad data.
fn corrupt_row(err: DomainError) -> DomainError {
	DomainError::Repository(format!("corrupt property row: {err}"))
}

/// Translate sqlx errors into domain errors. A unique violation is an honest
/// `Conflict`; everything else is an opaque `Repository`. A database failure is
/// never reported as a `Validation`.
fn map_sqlx_error(err: sqlx::Error) -> DomainError {
	if let sqlx::Error::Database(ref db_err) = err
		&& db_err.is_unique_violation()
	{
		return DomainError::Conflict("duplicate primary key".into());
	}
	DomainError::Repository(err.to_string())
}
