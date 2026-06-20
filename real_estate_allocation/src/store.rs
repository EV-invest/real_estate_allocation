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
	name TEXT NOT NULL,
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
	pub async fn open(db_path: &Path, data_dir: PathBuf) -> Result<Self, DomainError> {
		if let Some(parent) = db_path.parent() {
			std::fs::create_dir_all(parent).map_err(|e| DomainError::Repository(format!("create db dir: {e}")))?;
		}
		std::fs::create_dir_all(&data_dir).map_err(|e| DomainError::Repository(format!("create data dir: {e}")))?;

		let pool = SqlitePoolOptions::new()
			.connect(&format!("sqlite://{}?mode=rwc", db_path.display()))
			.await
			.map_err(map_sqlx_error)?;
		sqlx::query(SCHEMA).execute(&pool).await.map_err(map_sqlx_error)?;
		// Additive migration for DBs created before `name` existed. The only
		// expected failure is "duplicate column name" when it's already present
		// (fresh DBs get it from SCHEMA above), so the error is safe to drop.
		let _ = sqlx::query("ALTER TABLE properties ADD COLUMN name TEXT NOT NULL DEFAULT ''").execute(&pool).await;
		Ok(Self { pool, data_dir })
	}

	/// `./data/properties/<property_id>/<file_id>__<filename>`.
	pub fn file_path(&self, property_id: PropertyId, file_id: FileId, filename: &str) -> PathBuf {
		self.data_dir.join(property_id.raw().to_string()).join(format!("{}__{filename}", file_id.raw()))
	}
}

/// Our Quy Nhơn portfolio. The four already-built projects (one per developer
/// track from issue #3, plus the separately-selected Calla) are `Purchased` — the
/// holdings the portfolio view renders. Two under-construction towers (Q1 / Triton)
/// are `Purchasing` prospects. Marketing imagery (building shots, floor plans, unit
/// layouts) ships bundled and is written to disk on first run. Prices are
/// representative per-unit asking prices, converted from VND at ~25,000 VND/USD.
/// Idempotent: a non-empty DB is left untouched.
pub async fn seed(store: &SqliteStore) -> Result<(), DomainError> {
	let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM properties").fetch_one(&store.pool).await.map_err(map_sqlx_error)?.get("n");
	if count > 0 {
		return Ok(());
	}

	struct Seed {
		name: &'static str,
		coords: Coords,
		price: f64,
		state: PropertyState,
		research_url: &'static str,
		terms: &'static str,
		reasoning: &'static str,
		/// (filename-on-disk, content-type, bytes). Floor plans / unit layouts ride
		/// as `Pic` too so they render inline in the media gallery.
		pics: &'static [(&'static str, &'static str, &'static [u8])],
	}

	const JPG: &str = "image/jpeg";
	const PNG: &str = "image/png";

	let seeds = [
		Seed {
			name: "Quy Nhơn Melody",
			coords: Coords { lat: 13.7686, lng: 109.2278 },
			price: 96_000.0,
			state: PropertyState::Purchased,
			research_url: "https://www.hungthinhland.com/en/projects/detail/QUY-NHON-MELODY.html",
			terms: "Handed over early 2024 (topped out 2021, completed Dec 2023). 4-star seafront tourism-apartment standard.",
			reasoning: "Developer: Hưng Thịnh Group (with Kim Cúc). Two 35-floor towers (Tropical & Flamenco), 1,332 units + 21 shops on the An Dương Vương–Chương Dương beachfront. Representative 2-BR ≈ 2.4 tỷ VND. Beachfront short-stay rental demand behind an established national brand.",
			pics: &[("building.jpg", JPG, include_bytes!("../assets/seed/melody/building.jpg"))],
		},
		Seed {
			name: "Vina2 Panorama Quy Nhơn",
			coords: Coords { lat: 13.8050, lng: 109.2070 },
			price: 60_000.0,
			state: PropertyState::Purchased,
			research_url: "https://quynhonhomes.vn/can-ho-quy-nhon/can-ho-vina2-panorama/",
			terms: "Built and handed over from early 2024; residents occupying. Move-in available from 30% of unit value.",
			reasoning: "Developer: VINA2 (Investment & Construction JSC). 20 floors, 252 units (Studio–3BR) in the Đê Đông resettlement area, Nhơn Bình; riverside with pool and shophouse podium. ~22–26 tr/m². Lowest entry price of the four; Hà Thanh river / Thị Nại lagoon outlook.",
			pics: &[
				("building.png", PNG, include_bytes!("../assets/seed/vina2_panorama/building.png")),
				("real.jpg", JPG, include_bytes!("../assets/seed/vina2_panorama/real.jpg")),
				("floorplan.jpg", JPG, include_bytes!("../assets/seed/vina2_panorama/floorplan.jpg")),
			],
		},
		Seed {
			name: "Ecolife Riverside Quy Nhơn",
			coords: Coords { lat: 13.7720, lng: 109.2120 },
			price: 59_000.0,
			state: PropertyState::Purchased,
			research_url: "https://quynhonhomes.vn/can-ho-quy-nhon/ecolife-riverside/",
			terms: "Completed and handed over; red book (sổ hồng) issued — move in immediately.",
			reasoning: "Developer: Capital House. 27-floor single tower, 694 units on Điện Biên Phủ St along the Hà Thanh river. Green-building positioning; issued title lowers legal risk. Representative 2-BR ≈ 1.48 tỷ VND.",
			pics: &[
				("building.png", PNG, include_bytes!("../assets/seed/ecolife/building.png")),
				("real.jpg", JPG, include_bytes!("../assets/seed/ecolife/real.jpg")),
				("floorplan.png", PNG, include_bytes!("../assets/seed/ecolife/floorplan.png")),
			],
		},
		Seed {
			name: "The Calla (Calla Apartment Quy Nhơn)",
			coords: Coords { lat: 13.7542045, lng: 109.2073247 },
			price: 80_000.0,
			state: PropertyState::Purchased,
			research_url: "https://quynhonhomes.vn/can-ho-quy-nhon/calla-apartment-quy-nhon/",
			terms: "Completed, sổ hồng available. Bank financing up to 80% LTV with interest grace through handover.",
			reasoning: "Developer: Armo Investment & Development JSC. 29-floor tower (100m), 454 units + 13 shophouses in the Vũng Chua green urban area (QL1D, Ghềnh Ráng); ~800m to the beach. First garden-apartment in Quy Nhơn; mountain + sea + city views. Units 39–82m² (1–3BR), ~25–28 tr/m². Total project investment 563 tỷ VND.",
			pics: &[
				("building.jpg", JPG, include_bytes!("../assets/seed/calla/building.jpg")),
				("livingroom.jpg", JPG, include_bytes!("../assets/seed/calla/livingroom.jpg")),
				("floorplan.png", PNG, include_bytes!("../assets/seed/calla/floorplan.png")),
				("unit_87m2.png", PNG, include_bytes!("../assets/seed/calla/unit_87m2.png")),
			],
		},
		Seed {
			name: "Q1 Tower (Cadia Quy Nhơn)",
			coords: Coords { lat: 13.7710, lng: 109.2360 },
			price: 150_000.0,
			state: PropertyState::Purchasing,
			research_url: "https://q1-tower.vn/",
			terms: "Under construction (broke ground Jun 2022). Beachfront 5-star branded-residence; not yet handed over.",
			reasoning: "Developer: Phát Đạt (Ngô Mây Real Estate JSC, PDR). Diamond-plot 5,246m² at No.1 Ngô Mây, directly facing Quy Nhơn beach & Nguyễn Tất Thành square. 5-star tourism apartments + hotel operated to Wyndham standard, smart-home fitted. Branded-residence scarcity in the city centre; pre-handover entry.",
			pics: &[
				("building.jpg", JPG, include_bytes!("../assets/seed/q1_tower/building.jpg")),
				("render.jpg", JPG, include_bytes!("../assets/seed/q1_tower/render.jpg")),
				("livingroom.jpg", JPG, include_bytes!("../assets/seed/q1_tower/livingroom.jpg")),
			],
		},
		Seed {
			name: "Triton — Quy Nhơn Sky Residence",
			coords: Coords { lat: 13.7820, lng: 109.2190 },
			price: 110_000.0,
			state: PropertyState::Purchasing,
			research_url: "https://tritonquynhonsky.vn/",
			terms: "Under construction (launched 2025); pricing still being released. Not yet handed over.",
			reasoning: "Developer: Arita Corporation. 48-storey premium tower (4 basements) at 72B Tây Sơn, Quy Nhơn Nam. Tallest of the set; early-stage entry on a central arterial. Specs and pricing still firming up — treat valuation as provisional.",
			pics: &[
				("building.jpg", JPG, include_bytes!("../assets/seed/triton/building.jpg")),
				("location.jpg", JPG, include_bytes!("../assets/seed/triton/location.jpg")),
				("floorplan.jpg", JPG, include_bytes!("../assets/seed/triton/floorplan.jpg")),
			],
		},
	];

	for s in seeds {
		let id = PropertyId::new();
		sqlx::query("INSERT INTO properties (id, name, lat, lng, price, state, research_url, terms, deal_json, loan_json, additional_reasoning) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
			.bind(id.raw().to_string())
			.bind(s.name)
			.bind(s.coords.lat)
			.bind(s.coords.lng)
			.bind(s.price)
			.bind(s.state.as_str())
			.bind(s.research_url)
			.bind(s.terms)
			.bind(None::<String>)
			.bind(None::<String>)
			.bind(s.reasoning)
			.execute(&store.pool)
			.await
			.map_err(map_sqlx_error)?;

		for (filename, content_type, bytes) in s.pics {
			let fid = FileId::new();
			let path = store.file_path(id, fid, filename);
			if let Some(parent) = path.parent() {
				std::fs::create_dir_all(parent).map_err(|e| DomainError::Repository(format!("create file dir: {e}")))?;
			}
			std::fs::write(&path, bytes).map_err(|e| DomainError::Repository(format!("write seed pic: {e}")))?;
			store
				.add_file(PropertyFile {
					id: fid,
					property_id: id,
					kind: FileKind::Pic,
					filename: (*filename).into(),
					content_type: (*content_type).into(),
				})
				.await?;
		}
	}

	Ok(())
}
/// Row mirroring the `properties` table. Private so the domain model stays free
/// of persistence derives.
#[derive(FromRow)]
struct PropertyRow {
	id: String,
	name: String,
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
			name: row.name,
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
		let rows = sqlx::query_as::<_, PropertyRow>("SELECT id, name, lat, lng, price, state, research_url, terms, deal_json, loan_json, additional_reasoning FROM properties")
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
		let row = sqlx::query_as::<_, PropertyRow>("SELECT id, name, lat, lng, price, state, research_url, terms, deal_json, loan_json, additional_reasoning FROM properties WHERE id = ?")
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
