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
	domain::{Apartment, ApartmentStatus, Building, BuildingId, ConstructionStatus, Developer, FileId, FileKind, GooglePlace, Money, PropertyFile, PropertyState, ResearchUrl},
	error::DomainError,
};

/// A `Building` (with its apartments) is persisted whole as a serialised document, so
/// the Rust aggregate is the single source of truth — no column can drift out of sync
/// with the struct, and a lot cannot exist apart from its building. `developers` stays
/// a lookup table the `developer` reference is validated against on write (in
/// `SqliteStore::put`, since the reference now lives inside the JSON document).
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS developers (
	name TEXT PRIMARY KEY,
	note TEXT NOT NULL DEFAULT '',
	page TEXT
);
CREATE TABLE IF NOT EXISTS properties (
	id TEXT PRIMARY KEY,
	doc TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS property_files (
	id TEXT PRIMARY KEY,
	property_id TEXT NOT NULL REFERENCES properties(id),
	apt INTEGER,
	kind TEXT NOT NULL,
	filename TEXT NOT NULL,
	content_type TEXT NOT NULL
);
";

/// Leaf port over the `ev_lib` repository markers. No `UnitOfWork`: every write here
/// is a single row, so a transaction boundary would buy nothing.
#[async_trait]
pub trait BuildingRepository: Repository<Aggregate = Building> + Reader<Aggregate = Building> {
	async fn list(&self, spec: Option<&(dyn Specification<Building> + Sync)>) -> Result<Vec<Building>, DomainError>;
	async fn get(&self, id: BuildingId) -> Result<Option<Building>, DomainError>;
	/// Persist a building aggregate whole. The `developer` reference (if any) is
	/// validated against the developers table here — a dangling one is a `Validation`
	/// error, never silently written.
	async fn put(&self, b: &Building) -> Result<(), DomainError>;
	async fn add_file(&self, f: PropertyFile) -> Result<(), DomainError>;
	async fn list_files(&self, id: BuildingId) -> Result<Vec<PropertyFile>, DomainError>;
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

		// `foreign_keys(true)` per connection so `property_files.property_id` →
		// properties(id) is enforced by the DB, not just by discipline.
		let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
			.map_err(map_sqlx_error)?
			.create_if_missing(true)
			.foreign_keys(true);
		let pool = SqlitePoolOptions::new().connect_with(opts).await.map_err(map_sqlx_error)?;
		sqlx::query(SCHEMA).execute(&pool).await.map_err(map_sqlx_error)?;
		Ok(Self { pool, data_dir })
	}

	/// Building files at `./data/properties/<building_id>/<file_id>__<filename>`;
	/// per-lot files nested under `…/<building_id>/apt/<number>/…`.
	pub fn file_path(&self, building_id: BuildingId, apt: Option<u32>, file_id: FileId, filename: &str) -> PathBuf {
		let mut dir = self.data_dir.join(building_id.raw().to_string());
		if let Some(n) = apt {
			dir = dir.join("apt").join(n.to_string());
		}
		dir.join(format!("{}__{filename}", file_id.raw()))
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
		/// Headline target rate (% / yr), surfaced as "Target Yield". 0 = unset.
		target_appreciation: f64,
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
			target_appreciation: 18.0,
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
			target_appreciation: 12.0,
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
			target_appreciation: 10.0,
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
			target_appreciation: 14.0,
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
			price: Some(90_000.0), // provisional: pre-handover branded beachfront residence
			state: PropertyState::Purchasing,
			construction: ConstructionStatus::UnderConstruction,
			target_appreciation: 12.0,
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
			name: "TMS Luxury Hotel & Residence Quy Nhơn",
			place: "ChIJBVOIrolsbzERr_9ibfn1t-I", // Grand Hyams Hotel — the 5-star hotel occupying the TMS tower
			price: Some(76_000.0), // average apartment ≈ 1.9 tỷ VND @ ~25,000 VND/USD
			state: PropertyState::Interesting,
			construction: ConstructionStatus::Completed,
			target_appreciation: 12.0,
			developer: "TMS Group",
			research_url: "https://tms-quynhon.com",
			terms: "Completed and operating since 2022 (groundbreaking 2017); sổ hồng issuance underway. Condotel lease-back: 10%/yr guaranteed for 10 years (after 95% payment) then 85% owner / 15% operator, or 80/20 non-guaranteed, plus ~15 free nights/yr — treat the guarantee as a marketed target given VN condotel payout risk.",
			reasoning: "42-floor beachfront landmark (tallest in Quy Nhơn) at 28 Nguyễn Huệ, ~240m from the city beach. Single tower: ~746 condotel/tourist apartments (F4–F28) above 328 five-star rooms run as Grand Hyams Hotel (F29–F41) and an F41–42 sky bar. 1BR ~45–50m², 2BR ~65–71m² at ~29–36 tr/m²; representative apartment ≈ 1.9 tỷ VND (~$76k). Operating asset with branded-residence scarcity in the city centre.",
			pics: &[("building.jpg", JPG, include_bytes!("../assets/seed/tms/building.jpg"))],
		},
		Seed {
			name: "Triton — Quy Nhơn Sky Residence",
			place: "ChIJ7QjTJQBtbzERetFnxYHlsUM", // Triton Sky Residence, 72B Tây Sơn
			price: Some(72_000.0), // provisional: pricing still firming up (launched 2025)
			state: PropertyState::Purchasing,
			construction: ConstructionStatus::UnderConstruction,
			target_appreciation: 0.0,
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
	let developers: [(&str, &str, Option<&str>); 7] = [
		(
			"TMS Group",
			"Hà Nội–based diversified group (est. 2004; Công ty Cổ phần Tập đoàn TMS) spanning real estate, hospitality and trading. TMS Quy Nhơn is its flagship Bình Định tower.",
			Some("https://tms-quynhon.com"),
		),
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
		let id = BuildingId::new();
		let price = s.price.map(|p| Money::parse(p).expect("seed price is non-negative finite"));
		store
			.put(&Building {
				id,
				name: s.name.into(),
				place: GooglePlace::parse(s.place.into())?,
				construction: s.construction,
				// The rule's single source of truth: an unfinished building has no target.
				target_appreciation: match s.construction {
					ConstructionStatus::Completed => s.target_appreciation,
					ConstructionStatus::UnderConstruction => 0.0,
				},
				developer: Some(s.developer.into()),
				research_url: ResearchUrl::parse(s.research_url.into())?,
				terms: Some(s.terms.into()),
				deal: None,
				loan: None,
				additional_reasoning: Some(s.reasoning.into()),
				apartments: mock_apartments(id, s.state, price),
				coords: None,
			})
			.await?;

		for (filename, content_type, bytes) in s.pics {
			let fid = FileId::new();
			let path = store.file_path(id, None, fid, filename);
			if let Some(parent) = path.parent() {
				std::fs::create_dir_all(parent).map_err(|e| DomainError::Repository(format!("create file dir: {e}")))?;
			}
			std::fs::write(&path, bytes).map_err(|e| DomainError::Repository(format!("write seed pic: {e}")))?;
			store
				.add_file(PropertyFile {
					id: fid,
					building_id: id,
					apt: None,
					kind: FileKind::Pic,
					filename: (*filename).into(),
					content_type: (*content_type).into(),
				})
				.await?;
		}
	}

	Ok(())
}

/// Synthesise a stable lot roster for a building from its id, so the two-level model
/// is exercised end to end while a real per-lot source does not exist. `our_state` and
/// `base` are the building's representative single-unit figures: a handful of lots take
/// our relationship, the rest split Sold/Available, and prices jitter around `base`.
/// Called once at seed time — the result is persisted as real lots, not re-synthesised
/// per read. Logged once so the data is never silently taken as real.
fn mock_apartments(id: BuildingId, our_state: PropertyState, base: Option<Money>) -> Vec<Apartment> {
	static NOTICE: std::sync::Once = std::sync::Once::new();
	NOTICE.call_once(|| {
		dioxus::logger::tracing::warn!("apartment rosters are procedurally seeded from the building id — no real per-lot source yet");
	});

	let root = id.raw().as_u64_pair().0;
	let mix = |i: u64| -> u64 {
		let mut z = root.wrapping_add(i.wrapping_mul(0x9E3779B97F4A7C15));
		z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
		z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
		z ^ (z >> 31)
	};
	let total = 20 + (mix(0) % 21) as u32; // 20..=40 lots
	let ours = 1 + (mix(1) % 5) as u32; // 1..=5 of them ours
	let ours_status = match our_state {
		PropertyState::Purchased(ts) => ApartmentStatus::Purchased(ts),
		PropertyState::Purchasing => ApartmentStatus::Purchasing,
		PropertyState::Interesting => ApartmentStatus::Interesting,
	};
	(1..=total)
		.map(|n| {
			let status = if n <= ours {
				ours_status
			} else if mix(n as u64 * 3) % 100 < 55 {
				ApartmentStatus::Sold
			} else {
				ApartmentStatus::Available
			};
			let price = base.map(|m| {
				let f = 0.82 + (mix(n as u64 * 7) % 37) as f64 / 100.0; // 0.82..=1.18
				Money::parse((m.amount() * f).round()).expect("jittered price stays non-negative finite")
			});
			Apartment {
				number: n,
				status,
				price,
				price_series: Vec::new(),
			}
		})
		.collect()
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
	apt: Option<i64>,
	kind: String,
	filename: String,
	content_type: String,
}

impl TryFrom<FileRow> for PropertyFile {
	type Error = DomainError;

	fn try_from(row: FileRow) -> Result<Self, Self::Error> {
		Ok(Self {
			id: crate::domain::parse_file_id(&row.id).map_err(corrupt_row)?,
			building_id: crate::domain::parse_building_id(&row.property_id).map_err(corrupt_row)?,
			apt: row.apt.map(|n| n as u32),
			kind: row.kind.parse().map_err(|e| corrupt_row(DomainError::Repository(format!("unknown file kind: {e}"))))?,
			filename: row.filename,
			content_type: row.content_type,
		})
	}
}

impl Repository for SqliteStore {
	type Aggregate = Building;
}

impl Reader for SqliteStore {
	type Aggregate = Building;
}

#[async_trait]
impl BuildingRepository for SqliteStore {
	/// Rows are loaded then filtered in Rust via `spec.holds`. The spec engine is
	/// in-memory by design; SQL pushdown is explicitly descoped.
	async fn list(&self, spec: Option<&(dyn Specification<Building> + Sync)>) -> Result<Vec<Building>, DomainError> {
		let rows = sqlx::query("SELECT doc FROM properties").fetch_all(&self.pool).await.map_err(map_sqlx_error)?;
		let mut out = Vec::new();
		for row in rows {
			let b = deserialize_building(row.get("doc"))?;
			if spec.map(|s| Specification::holds(s, &b)).unwrap_or(true) {
				out.push(b);
			}
		}
		Ok(out)
	}

	async fn get(&self, id: BuildingId) -> Result<Option<Building>, DomainError> {
		let row = sqlx::query("SELECT doc FROM properties WHERE id = ?")
			.bind(id.raw().to_string())
			.fetch_optional(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		row.map(|r| deserialize_building(r.get("doc"))).transpose()
	}

	async fn put(&self, b: &Building) -> Result<(), DomainError> {
		if let Some(dev) = &b.developer
			&& self.get_developer(dev).await?.is_none()
		{
			return Err(DomainError::Validation(format!("unknown developer: {dev}")));
		}
		let doc = serde_json::to_string(b).map_err(|e| DomainError::Repository(format!("serialize building: {e}")))?;
		sqlx::query("INSERT INTO properties (id, doc) VALUES (?, ?)")
			.bind(b.id.raw().to_string())
			.bind(doc)
			.execute(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
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
		sqlx::query("INSERT INTO property_files (id, property_id, apt, kind, filename, content_type) VALUES (?, ?, ?, ?, ?, ?)")
			.bind(f.id.raw().to_string())
			.bind(f.building_id.raw().to_string())
			.bind(f.apt.map(|n| n as i64))
			.bind(f.kind.as_ref())
			.bind(&f.filename)
			.bind(&f.content_type)
			.execute(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	/// All files for a building, lot-level and building-level alike; the caller
	/// (`api::list_files`) narrows to the active level via each file's `apt`.
	async fn list_files(&self, id: BuildingId) -> Result<Vec<PropertyFile>, DomainError> {
		let rows = sqlx::query_as::<_, FileRow>("SELECT id, property_id, apt, kind, filename, content_type FROM property_files WHERE property_id = ?")
			.bind(id.raw().to_string())
			.fetch_all(&self.pool)
			.await
			.map_err(map_sqlx_error)?;
		rows.into_iter().map(PropertyFile::try_from).collect()
	}
}

/// Reconstruct a building from its stored document. The aggregate was serialised from a
/// valid value on the way in, so a parse failure means the row is corrupt — a repository
/// error, never a client-facing validation, and never silently defaulted away.
fn deserialize_building(doc: &str) -> Result<Building, DomainError> {
	serde_json::from_str(doc).map_err(|e| corrupt_row(DomainError::Repository(format!("deserialize building: {e}"))))
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
