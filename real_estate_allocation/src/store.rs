use std::{
	path::{Path, PathBuf},
	str::FromStr as _,
};

use async_trait::async_trait;
use ev_lib::architecture::{Reader, Repository, Specification};
use sqlx::{
	FromRow, Row as _, SqlitePool,
	sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::{
	domain::{
		ConstructionStatus, DealStructure, Developer, FileId, FileKind, GooglePlace, LoanRates, Money, Property, PropertyFile, PropertyId, PropertyState, PropertyStateKind, ResearchUrl,
	},
	error::DomainError,
};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS developers (
	name TEXT PRIMARY KEY,
	note TEXT NOT NULL DEFAULT '',
	page TEXT
);
CREATE TABLE IF NOT EXISTS properties (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	place_id TEXT NOT NULL,
	price REAL,
	state TEXT NOT NULL,
	purchased_at TEXT,
	construction TEXT NOT NULL,
	developer TEXT REFERENCES developers(name),
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

/// Leaf port over the `ev_lib` repository markers. No `UnitOfWork`: every write here
/// is a single row, so a transaction boundary would buy nothing.
#[async_trait]
pub trait PropertyRepository: Repository<Aggregate = Property> + Reader<Aggregate = Property> {
	async fn list(&self, spec: Option<&(dyn Specification<Property> + Sync)>) -> Result<Vec<Property>, DomainError>;
	async fn get(&self, id: PropertyId) -> Result<Option<Property>, DomainError>;
	async fn add_file(&self, f: PropertyFile) -> Result<(), DomainError>;
	async fn list_files(&self, id: PropertyId) -> Result<Vec<PropertyFile>, DomainError>;
	async fn get_developer(&self, name: &str) -> Result<Option<Developer>, DomainError>;
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

		// `foreign_keys(true)` per connection so the `developer` → developers(name)
		// reference is enforced by the DB, not just by seed discipline.
		let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
			.map_err(map_sqlx_error)?
			.create_if_missing(true)
			.foreign_keys(true);
		let pool = SqlitePoolOptions::new().connect_with(opts).await.map_err(map_sqlx_error)?;
		sqlx::query(SCHEMA).execute(&pool).await.map_err(map_sqlx_error)?;
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
/// layouts) ships bundled and is written to disk on first run. Prices, where
/// known, are representative per-unit asking prices converted from VND at
/// ~25,000 VND/USD; the under-construction towers have none yet (`None`).
/// Idempotent: a non-empty DB is left untouched.
pub async fn seed(store: &SqliteStore) -> Result<(), DomainError> {
	let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM properties").fetch_one(&store.pool).await.map_err(map_sqlx_error)?.get("n");
	if count > 0 {
		return Ok(());
	}

	struct Seed {
		name: &'static str,
		place: &'static str,
		/// `None` where we have no real number yet (the two under-construction towers).
		price: Option<f64>,
		state: PropertyState,
		construction: ConstructionStatus,
		developer: &'static str,
		research_url: &'static str,
		terms: &'static str,
		reasoning: &'static str,
		/// (filename-on-disk, content-type, bytes). Floor plans / unit layouts ride
		/// as `Pic` too so they render inline in the media gallery.
		pics: &'static [(&'static str, &'static str, &'static [u8])],
	}

	const JPG: &str = "image/jpeg";
	const PNG: &str = "image/png";

	// Purchase instants are relative to now so the demo holds regardless of when the
	// DB is seeded. Melody is deliberately old: its mock series ends long before today,
	// so the chart shows the dotted "stale" projection out to the present.
	let now = jiff::Timestamp::now();
	let weeks_ago = |w: i64| jiff::Timestamp::from_second(now.as_second() - w * 7 * 24 * 3600).expect("seed purchase date in range");

	let seeds = [
		Seed {
			name: "Quy Nhơn Melody",
			place: "ChIJOYTnE0ZtbzERRZnbRLfEIU8", // Căn Hộ Quy Nhơn Melody
			price: Some(96_000.0),
			state: PropertyState::Purchased(weeks_ago(54)),
			construction: ConstructionStatus::Completed,
			developer: "Hưng Thịnh",
			research_url: "https://www.hungthinhland.com/en/projects/detail/QUY-NHON-MELODY.html",
			terms: "Handed over early 2024 (topped out 2021, completed Dec 2023). 4-star seafront tourism-apartment standard.",
			reasoning: "Two 35-floor towers (Tropical & Flamenco) with Kim Cúc, 1,332 units + 21 shops on the An Dương Vương–Chương Dương beachfront. Representative 2-BR ≈ 2.4 tỷ VND. Beachfront short-stay rental demand behind an established national brand.",
			pics: &[("building.jpg", JPG, include_bytes!("../assets/seed/melody/building.jpg"))],
		},
		Seed {
			name: "Vina2 Panorama Quy Nhơn",
			place: "ChIJTX0aij9rbzERvgC-Y2tqNxw", // VINA2 Panorama
			price: Some(60_000.0),
			state: PropertyState::Purchased(weeks_ago(20)),
			construction: ConstructionStatus::Completed,
			developer: "VINA2",
			research_url: "https://quynhonhomes.vn/can-ho-quy-nhon/can-ho-vina2-panorama/",
			terms: "Built and handed over from early 2024; residents occupying. Move-in available from 30% of unit value.",
			reasoning: "20 floors, 252 units (Studio–3BR) in the Đê Đông resettlement area, Nhơn Bình; riverside with pool and shophouse podium. ~22–26 tr/m². Lowest entry price of the four; Hà Thanh river / Thị Nại lagoon outlook.",
			pics: &[
				("building.png", PNG, include_bytes!("../assets/seed/vina2_panorama/building.png")),
				("real.jpg", JPG, include_bytes!("../assets/seed/vina2_panorama/real.jpg")),
				("floorplan.jpg", JPG, include_bytes!("../assets/seed/vina2_panorama/floorplan.jpg")),
			],
		},
		Seed {
			name: "Ecolife Riverside Quy Nhơn",
			place: "ChIJGb0UXFRrbzER9Ym2RZ7csFE", // Ecolife Riverside Quy Nhơn
			price: Some(59_000.0),
			state: PropertyState::Purchased(weeks_ago(14)),
			construction: ConstructionStatus::Completed,
			developer: "Capital House",
			research_url: "https://quynhonhomes.vn/can-ho-quy-nhon/ecolife-riverside/",
			terms: "Completed and handed over; red book (sổ hồng) issued — move in immediately.",
			reasoning: "27-floor single tower, 694 units on Điện Biên Phủ St along the Hà Thanh river. Green-building positioning; issued title lowers legal risk. Representative 2-BR ≈ 1.48 tỷ VND.",
			pics: &[
				("building.png", PNG, include_bytes!("../assets/seed/ecolife/building.png")),
				("real.jpg", JPG, include_bytes!("../assets/seed/ecolife/real.jpg")),
				("floorplan.png", PNG, include_bytes!("../assets/seed/ecolife/floorplan.png")),
			],
		},
		Seed {
			name: "The Calla (Calla Apartment Quy Nhơn)",
			place: "ChIJt5jJjCxtbzERKfYIEUv0i-A", // The Calla (matches the shared map pin)
			price: Some(80_000.0),
			state: PropertyState::Purchased(weeks_ago(9)),
			construction: ConstructionStatus::Completed,
			developer: "Armo",
			research_url: "https://quynhonhomes.vn/can-ho-quy-nhon/calla-apartment-quy-nhon/",
			terms: "Completed, sổ hồng available. Bank financing up to 80% LTV with interest grace through handover.",
			reasoning: "29-floor tower (100m), 454 units + 13 shophouses in the Vũng Chua green urban area (QL1D, Ghềnh Ráng); ~800m to the beach. First garden-apartment in Quy Nhơn; mountain + sea + city views. Units 39–82m² (1–3BR), ~25–28 tr/m². Total project investment 563 tỷ VND.",
			pics: &[
				("building.jpg", JPG, include_bytes!("../assets/seed/calla/building.jpg")),
				("livingroom.jpg", JPG, include_bytes!("../assets/seed/calla/livingroom.jpg")),
				("floorplan.png", PNG, include_bytes!("../assets/seed/calla/floorplan.png")),
				("unit_87m2.png", PNG, include_bytes!("../assets/seed/calla/unit_87m2.png")),
			],
		},
		Seed {
			name: "Q1 Tower (Cadia Quy Nhơn)",
			place: "ChIJDQMq0yFtbzERY32pkB70paY", // Q1 Tower Quy Nhơn, 1 Ngô Mây
			price: None,
			state: PropertyState::Purchasing,
			construction: ConstructionStatus::UnderConstruction,
			developer: "Phát Đạt",
			research_url: "https://q1-tower.vn/",
			terms: "Under construction (broke ground Jun 2022). Beachfront 5-star branded-residence; not yet handed over.",
			reasoning: "Diamond-plot 5,246m² at No.1 Ngô Mây, directly facing Quy Nhơn beach & Nguyễn Tất Thành square. 5-star tourism apartments + hotel operated to Wyndham standard, smart-home fitted. Branded-residence scarcity in the city centre; pre-handover entry.",
			pics: &[
				("building.jpg", JPG, include_bytes!("../assets/seed/q1_tower/building.jpg")),
				("render.jpg", JPG, include_bytes!("../assets/seed/q1_tower/render.jpg")),
				("livingroom.jpg", JPG, include_bytes!("../assets/seed/q1_tower/livingroom.jpg")),
			],
		},
		Seed {
			name: "Triton — Quy Nhơn Sky Residence",
			place: "ChIJ7QjTJQBtbzERetFnxYHlsUM", // Triton Sky Residence, 72B Tây Sơn
			price: None,
			state: PropertyState::Purchasing,
			construction: ConstructionStatus::UnderConstruction,
			developer: "Arita",
			research_url: "https://tritonquynhonsky.vn/",
			terms: "Under construction (launched 2025); pricing still being released. Not yet handed over.",
			reasoning: "48-storey premium tower (4 basements) at 72B Tây Sơn, Quy Nhơn Nam. Tallest of the set; early-stage entry on a central arterial. Specs and pricing still firming up — treat valuation as provisional.",
			pics: &[
				("building.jpg", JPG, include_bytes!("../assets/seed/triton/building.jpg")),
				("location.jpg", JPG, include_bytes!("../assets/seed/triton/location.jpg")),
				("floorplan.jpg", JPG, include_bytes!("../assets/seed/triton/floorplan.jpg")),
			],
		},
	];

	// Developers first: the FK on properties.developer requires them to exist.
	let developers: [(&str, &str, Option<&str>); 6] = [
		(
			"Hưng Thịnh",
			"Large national developer; ecosystem includes Hưng Thịnh Land & Incons. 2022–23 liquidity stress — watch counterparty risk.",
			Some("https://hungthinhcorp.com.vn/"),
		),
		(
			"VINA2",
			"Listed contractor-developer (HNX: VC2) that builds its own projects. Smaller balance sheet.",
			Some("https://vina2.com.vn/"),
		),
		("Capital House", "Hà Nội green-building specialist behind the Ecolife / Ecohome brands.", None),
		("Armo", "Local Bình Định developer; The Calla is its flagship tower.", None),
		(
			"Phát Đạt",
			"Major listed developer (HOSE: PDR), HCMC-centric. Bình Định pipeline slipped to ~2027.",
			Some("https://phatdat.com.vn/"),
		),
		("Arita", "Local developer; Triton is early-stage with a limited track record.", None),
	];
	for (name, note, page) in developers {
		sqlx::query("INSERT INTO developers (name, note, page) VALUES (?, ?, ?)")
			.bind(name)
			.bind(note)
			.bind(page)
			.execute(&store.pool)
			.await
			.map_err(map_sqlx_error)?;
	}

	for s in seeds {
		let id = PropertyId::new();
		sqlx::query("INSERT INTO properties (id, name, place_id, price, state, purchased_at, construction, developer, research_url, terms, deal_json, loan_json, additional_reasoning) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
			.bind(id.raw().to_string())
			.bind(s.name)
			.bind(s.place)
			.bind(s.price)
			.bind(s.state.kind().as_ref())
			.bind(match s.state {
				PropertyState::Purchased(ts) => Some(ts.to_string()),
				_ => None,
			})
			.bind(s.construction.as_ref())
			.bind(s.developer)
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
	place_id: String,
	price: Option<f64>,
	state: String,
	purchased_at: Option<String>,
	construction: String,
	developer: Option<String>,
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
			place: GooglePlace::parse(row.place_id).map_err(corrupt_row)?,
			price: row.price.map(Money::parse).transpose().map_err(corrupt_row)?,
			state: match row
				.state
				.parse::<PropertyStateKind>()
				.map_err(|e| corrupt_row(DomainError::Repository(format!("unknown property state: {e}"))))?
			{
				PropertyStateKind::Purchased => {
					let raw = row
						.purchased_at
						.ok_or_else(|| corrupt_row(DomainError::Repository("purchased property row missing purchased_at".into())))?;
					PropertyState::Purchased(raw.parse().map_err(|e| corrupt_row(DomainError::Repository(format!("bad purchased_at: {e}"))))?)
				}
				PropertyStateKind::Interesting => PropertyState::Interesting,
				PropertyStateKind::Purchasing => PropertyState::Purchasing,
			},
			construction: row
				.construction
				.parse()
				.map_err(|e| corrupt_row(DomainError::Repository(format!("unknown construction status: {e}"))))?,
			developer: row.developer,
			research_url: ResearchUrl::parse(row.research_url).map_err(corrupt_row)?,
			terms: row.terms,
			deal,
			loan,
			additional_reasoning: row.additional_reasoning,
			price_series: Vec::new(),
			coords: None,
		})
	}
}

#[derive(FromRow)]
struct DeveloperRow {
	name: String,
	note: String,
	page: Option<String>,
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
			kind: row.kind.parse().map_err(|e| corrupt_row(DomainError::Repository(format!("unknown file kind: {e}"))))?,
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
		let rows = sqlx::query_as::<_, PropertyRow>(
			"SELECT id, name, place_id, price, state, purchased_at, construction, developer, research_url, terms, deal_json, loan_json, additional_reasoning FROM properties",
		)
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
		let row = sqlx::query_as::<_, PropertyRow>(
			"SELECT id, name, place_id, price, state, purchased_at, construction, developer, research_url, terms, deal_json, loan_json, additional_reasoning FROM properties WHERE id = ?",
		)
		.bind(id.raw().to_string())
		.fetch_optional(&self.pool)
		.await
		.map_err(map_sqlx_error)?;
		row.map(Property::try_from).transpose()
	}

	async fn get_developer(&self, name: &str) -> Result<Option<Developer>, DomainError> {
		let row = sqlx::query_as::<_, DeveloperRow>("SELECT name, note, page FROM developers WHERE name = ?")
			.bind(name)
			.fetch_optional(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		Ok(row.map(|r| Developer {
			name: r.name,
			note: r.note,
			page: r.page,
		}))
	}

	async fn add_file(&self, f: PropertyFile) -> Result<(), DomainError> {
		sqlx::query("INSERT INTO property_files (id, property_id, kind, filename, content_type) VALUES (?, ?, ?, ?, ?)")
			.bind(f.id.raw().to_string())
			.bind(f.property_id.raw().to_string())
			.bind(f.kind.as_ref())
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
