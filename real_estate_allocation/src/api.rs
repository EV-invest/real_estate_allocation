#[cfg(not(target_arch = "wasm32"))]
use std::collections::BTreeMap;

use dioxus::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use dioxus::server::axum::{Extension, Json, extract::Path};

use crate::domain::{Building, BuildingId, FileKind, PropertyFile, PropertyStateKind};

#[cfg(not(target_arch = "wasm32"))]
const CACHE_TTL_SECS: i64 = 30 * 24 * 3600;
/// The committed seeds (breakpoint → arrangement, keyed like `layout_path`), baked
/// into the binary so a fresh prod volume (no saved `layout_path` yet) still opens
/// onto the curated layout instead of the bare built-in seed. Pressing `s` writes
/// `layout_path`, which overrides this.
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_LAYOUT: &str = include_str!("../assets/dashboard_layout.json");
/// The only client↔server seam. Each `#[server]` fn runs on the host, pulling the
/// `SqliteStore` / `AppConfig` out of the per-request axum extension (attached in
/// `main`), and is called as an async fn from the wasm client.

/// Shared host state, attached to every request as an axum `Extension` in `main`.
/// Server-only: the wasm client never constructs it.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct AppState {
	pub store: crate::store::SqliteStore,
	pub config: crate::config::AppConfig,
}
#[server]
pub async fn list_buildings(filter: Option<Vec<PropertyStateKind>>) -> Result<Vec<Building>, ServerFnError> {
	use ev_lib::architecture::Specification;

	use crate::{domain::InState, store::BuildingRepository};

	let state = app_state().await?;

	// A building is in if any of its lots is ours in one of the requested kinds; the
	// `Fn(&T)->bool` blanket impl makes the disjunction itself a `Specification`.
	let states = filter.unwrap_or_else(|| vec![PropertyStateKind::Purchased]);
	let spec = move |b: &Building| states.iter().any(|s| InState(*s).holds(b));
	let mut buildings = state.store.list(Some(&spec)).await.map_err(to_server_err)?;
	for b in buildings.iter_mut() {
		b.coords = resolve_coords(b.id, &b.place, &state.config).await;
	}
	Ok(buildings)
}
#[server]
pub async fn get_building(id: BuildingId) -> Result<Option<Building>, ServerFnError> {
	let state = app_state().await?;
	let mut building = enrich_building(&state, id).await.map_err(to_server_err)?;
	if let Some(b) = building.as_mut() {
		b.coords = resolve_coords(id, &b.place, &state.config).await;
	}
	Ok(building)
}
/// Plain-HTTP sibling of `get_building` for the CSR microfrontend embed (which has
/// no `#[server]`/fullstack runtime). Same enriched `Building` JSON on the wire;
/// always 200 with `null` body on a bad id or repo fault (logged) — the embed maps
/// `null` to its "ERR"/loading fallback.
#[cfg(not(target_arch = "wasm32"))]
pub async fn building_json(Extension(state): Extension<AppState>, Path(id): Path<String>) -> Json<Option<Building>> {
	let building = match crate::domain::parse_building_id(&id) {
		Ok(bid) => match enrich_building(&state, bid).await {
			Ok(b) => b,
			Err(e) => {
				dioxus::logger::tracing::error!(%e, building = %id, "embed building_json: enrich failed");
				None
			}
		},
		Err(e) => {
			dioxus::logger::tracing::warn!(%e, raw = %id, "embed building_json: bad building id");
			None
		}
	};
	Json(building)
}
#[server]
pub async fn get_developer(name: String) -> Result<Option<crate::domain::Developer>, ServerFnError> {
	use crate::store::BuildingRepository;

	let store = app_state().await?.store;
	store.get_developer(&name).await.map_err(to_server_err)
}
#[server]
pub async fn list_files(id: BuildingId, appt: Option<u32>) -> Result<Vec<PropertyFile>, ServerFnError> {
	use crate::store::BuildingRepository;

	let store = app_state().await?.store;
	let files = store.list_files(id).await.map_err(to_server_err)?;
	// Building level shows building files only; a lot shows just its own.
	Ok(files.into_iter().filter(|f| f.apt == appt).collect())
}
#[server]
pub async fn upload_file(
	building_id: BuildingId,
	appt: Option<u32>,
	kind: FileKind,
	filename: String,
	content_type: String,
	bytes: Vec<u8>,
	token: String,
) -> Result<PropertyFile, ServerFnError> {
	use secrecy::ExposeSecret as _;

	use crate::store::BuildingRepository;

	let AppState { store, config: cfg } = app_state().await?;
	if token != cfg.admin_token.expose_secret() {
		return Err(ServerFnError::new("not authorized to upload"));
	}

	let file_id = crate::domain::FileId::new();
	let path = store.file_path(building_id, appt, file_id, &filename);
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent).map_err(|e| ServerFnError::new(format!("create dir: {e}")))?;
	}
	std::fs::write(&path, &bytes).map_err(|e| ServerFnError::new(format!("write file: {e}")))?;

	let file = PropertyFile {
		id: file_id,
		building_id,
		apt: appt,
		kind,
		filename,
		content_type,
	};
	store.add_file(file.clone()).await.map_err(to_server_err)?;
	Ok(file)
}
#[server]
pub async fn file_bytes(id: BuildingId, appt: Option<u32>, file_id: crate::domain::FileId, filename: String) -> Result<Vec<u8>, ServerFnError> {
	let store = app_state().await?.store;
	let path = store.file_path(id, appt, file_id, &filename);
	std::fs::read(&path).map_err(|e| ServerFnError::new(format!("read file: {e}")))
}
#[server]
pub async fn am_i_admin(token: String) -> Result<bool, ServerFnError> {
	use secrecy::ExposeSecret as _;

	let cfg = app_state().await?.config;
	Ok(!token.is_empty() && token == cfg.admin_token.expose_secret())
}
/// Persist the current dock arrangement as its band group's global seed (in the map
/// at `config.layout_path`). An `xl`-group save also becomes the `default` seed, the
/// one groups without their own save open onto.
#[server]
pub async fn save_default_layout(json: String, breakpoint: dockview_dioxus::Breakpoint) -> Result<(), ServerFnError> {
	let layout: serde_json::Value = serde_json::from_str(&json).map_err(|e| ServerFnError::new(format!("layout not json: {e}")))?;
	let path = app_state().await?.config.layout_path;
	let path = path.as_ref();
	let mut seeds = match std::fs::read_to_string(path) {
		Ok(s) => parse_seeds(&s).map_err(ServerFnError::new)?,
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
		Err(e) => return Err(ServerFnError::new(format!("read layout: {e}"))),
	};
	let key = seed_key(breakpoint);
	if key == "xl" {
		seeds.insert("default".into(), layout.clone());
	}
	seeds.insert(key.into(), layout);
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent).map_err(|e| ServerFnError::new(format!("create layout dir: {e}")))?;
	}
	std::fs::write(path, serde_json::to_string(&seeds).expect("seeds map serializes")).map_err(|e| ServerFnError::new(format!("write layout: {e}")))?;
	Ok(())
}

/// This band group's saved seed, falling back to the `default` one, from the saved
/// map (or the committed `DEFAULT_LAYOUT` when none has been saved on this volume
/// yet). A genuine read failure surfaces as an error.
#[server]
pub async fn load_default_layout(breakpoint: dockview_dioxus::Breakpoint) -> Result<Option<String>, ServerFnError> {
	let path = app_state().await?.config.layout_path;
	let seeds = match std::fs::read_to_string(path.as_ref()) {
		Ok(s) => parse_seeds(&s).map_err(ServerFnError::new)?,
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => parse_seeds(DEFAULT_LAYOUT).expect("committed seeds parse"),
		Err(e) => return Err(ServerFnError::new(format!("read layout: {e}"))),
	};
	Ok(seeds.get(seed_key(breakpoint)).or_else(|| seeds.get("default")).map(|v| v.to_string()))
}

#[server]
pub async fn maps_api_key() -> Result<String, ServerFnError> {
	use secrecy::ExposeSecret as _;

	let cfg = app_state().await?.config;
	Ok(cfg.maps_api_key.expose_secret().to_string())
}
/// Seeds are kept coarser than the measured band: xl/lg share one arrangement, so do
/// sm/xs. Not gated server-side — the client keys its save toast off the same grouping.
pub(crate) fn seed_key(breakpoint: dockview_dioxus::Breakpoint) -> &'static str {
	use dockview_dioxus::Breakpoint::*;
	match breakpoint {
		Xl | Lg => "xl",
		Md => "md",
		Sm | Xs => "sm",
	}
}

/// `BTreeMap` so the file's bytes are deterministic — it participates in the sync
/// snapshot's content hash. A `version` top-level key marks the pre-seeds format
/// (one bare arrangement): adopt it as `default`.
#[cfg(not(target_arch = "wasm32"))]
fn parse_seeds(s: &str) -> Result<BTreeMap<String, serde_json::Value>, String> {
	let serde_json::Value::Object(m) = serde_json::from_str(s).map_err(|e| format!("layout file: {e}"))? else {
		return Err("layout file: expected an object".into());
	};
	if m.contains_key("version") {
		return Ok([("default".to_string(), serde_json::Value::Object(m))].into());
	}
	// An earlier scheme kept a seed per band; fold `lg`/`xs` into the group that
	// subsumed them, the group's own save winning.
	let mut m = m;
	for (old, new) in [("lg", "xl"), ("xs", "sm")] {
		if let Some(v) = m.remove(old) {
			m.entry(new).or_insert(v);
		}
	}
	const KEYS: [&str; 4] = ["default", "xl", "md", "sm"];
	if let Some(k) = m.keys().find(|k| !KEYS.contains(&k.as_str())) {
		return Err(format!("layout file: unknown seed key `{k}`"));
	}
	Ok(m.into_iter().collect())
}
/// `store.get` + the per-lot `price_series` synthesis (the basis for
/// `appreciation_yoy`). No coord resolution — callers that need a map pin add it
/// (`get_building`); the embed route deliberately skips it (no Places call).
#[cfg(not(target_arch = "wasm32"))]
async fn enrich_building(state: &AppState, id: BuildingId) -> Result<Option<Building>, crate::error::DomainError> {
	use crate::{domain::ApartmentStatus, store::BuildingRepository};

	let mut building = state.store.get(id).await?;
	if let Some(b) = building.as_mut() {
		let root = id.raw().as_u64_pair().0;
		for a in b.apartments.iter_mut() {
			let seed = root ^ (a.number as u64).wrapping_mul(0x9e3779b97f4a7c15);
			let purchased = match a.status {
				ApartmentStatus::Purchased(ts) => Some(ts),
				_ => None,
			};
			a.price_series = mock_series(seed, purchased);
		}
	}
	Ok(building)
}
/// Epoch second until which the Places API has told us to back off (via a `429`
/// `Retry-After`). Process-global because the whole API key is what's throttled,
/// not any one place. ponytail: in-memory, so a server restart costs one probe
/// request that simply re-arms it; persist to disk if that ever matters.
#[cfg(not(target_arch = "wasm32"))]
static PLACES_BLOCKED_UNTIL: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

//HACK: see `main` — `LaunchBuilder::with_context` doesn't reach server fns in
// dioxus-server 0.7.9, so we read our state from the request extension instead.
// `FullstackContext` is the one handle available in both the SSR render path and
// the server-fn POST path.

/// Deterministic mock value series, seeded so it is stable per lot. Anchored to the
/// purchase instant (a few weeks of pre-purchase tracking, then a fixed run of weekly
/// estimates clipped to now). A long-ago purchase therefore produces a series that ends
/// well before today — the chart fills that tail with a dotted projection. Lots we don't
/// own anchor to a trailing window.
#[cfg(not(target_arch = "wasm32"))]
fn mock_series(seed: u64, purchased_at: Option<jiff::Timestamp>) -> Vec<(jiff::Timestamp, f64)> {
	const WEEK: i64 = 7 * 24 * 3600;
	const LEAD: i64 = 8; // weeks of pre-purchase estimate tracking (drawn dimmed)
	const SPAN: usize = 30; // weeks of estimates generated

	let now = jiff::Timestamp::now();
	let anchor = match purchased_at {
		Some(ts) => ts,
		None => jiff::Timestamp::from_second(now.as_second() - (SPAN as i64) * WEEK).expect("trailing window in range"),
	};
	let start = anchor.as_second() - LEAD * WEEK;

	let walk = v_utils::distributions::laplace_random_walk(100.0, SPAN, 0.1, 0.0, Some(seed));
	walk.into_iter()
		.enumerate()
		// ~1 week in 8 has no estimate (stable per seed): a genuine gap the line spans.
		.filter(|(i, _)| (seed as usize).wrapping_add(i * 7) % 8 != 0)
		.map(|(i, v)| (start + i as i64 * WEEK, v))
		.take_while(|(t, _)| *t <= now.as_second())
		.map(|(t, v)| (jiff::Timestamp::from_second(t).expect("week within range"), v))
		.collect()
}
/// On-disk pin cache at `<data_dir>/<id>/place.json`. `place_id` is stored so a
/// changed place invalidates the entry; `fetched_at` drives the monthly refresh.
#[cfg(not(target_arch = "wasm32"))]
#[derive(serde::Deserialize, serde::Serialize)]
struct CachedPlace {
	place_id: String,
	coords: crate::domain::Coords,
	fetched_at: jiff::Timestamp,
}

/// Resolve a property's pin, reading the disk cache and only hitting Google's
/// Places API when the entry is missing, stale (>1 month), or points at a
/// different place. A failed resolve yields `None` (no pin) rather than failing
/// the whole listing — one unresolvable place must not blank the map.
#[cfg(not(target_arch = "wasm32"))]
async fn resolve_coords(id: BuildingId, place: &crate::domain::GooglePlace, cfg: &crate::config::AppConfig) -> Option<crate::domain::Coords> {
	use secrecy::ExposeSecret as _;

	let path = cfg.data_dir.as_ref().join(id.raw().to_string()).join("place.json");

	if let Ok(bytes) = std::fs::read(&path) {
		// A corrupt/old-schema cache file is simply ignored and re-fetched below.
		if let Ok(c) = serde_json::from_slice::<CachedPlace>(&bytes) {
			let age = jiff::Timestamp::now().as_second() - c.fetched_at.as_second();
			if c.place_id == place.as_str() && age < CACHE_TTL_SECS {
				return Some(c.coords);
			}
		}
	}

	use std::sync::atomic::Ordering::Relaxed;
	let now = jiff::Timestamp::now().as_second();
	// Honour an outstanding 429 back-off: skip the request entirely (no pin, no log
	// spam) until the window the API gave us has elapsed.
	if now < PLACES_BLOCKED_UNTIL.load(Relaxed) {
		return None;
	}

	let coords = match fetch_place(place.as_str(), cfg.maps_api_key.expose_secret()).await {
		Ok(c) => c,
		Err(PlaceError::RateLimited { retry_after_secs }) => {
			PLACES_BLOCKED_UNTIL.store(now + retry_after_secs, Relaxed);
			dioxus::logger::tracing::warn!(retry_after_secs, "places rate limited (429); backing off");
			return None;
		}
		// A non-429 failure drops just this pin; the rest of the listing is unaffected.
		Err(PlaceError::Other(e)) => {
			dioxus::logger::tracing::warn!(%e, place = place.as_str(), "place resolve failed");
			return None;
		}
	};

	let cached = CachedPlace {
		place_id: place.as_str().to_owned(),
		coords,
		fetched_at: jiff::Timestamp::now(),
	};
	let write = (|| -> std::io::Result<()> {
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		std::fs::write(&path, serde_json::to_vec_pretty(&cached).expect("CachedPlace serialises"))
	})();
	// A cache-write failure only costs us next request's fetch; the coords are still good.
	if let Err(e) = write {
		dioxus::logger::tracing::warn!(%e, "cache place coords failed");
	}
	Some(coords)
}

#[cfg(not(target_arch = "wasm32"))]
enum PlaceError {
	/// `429` — carries the server's `Retry-After` (seconds), defaulted when absent.
	RateLimited {
		retry_after_secs: i64,
	},
	Other(String),
}

/// One GET against the Places API (New): `location` field only.
#[cfg(not(target_arch = "wasm32"))]
async fn fetch_place(place_id: &str, key: &str) -> Result<crate::domain::Coords, PlaceError> {
	#[derive(serde::Deserialize)]
	struct Location {
		latitude: f64,
		longitude: f64,
	}
	#[derive(serde::Deserialize)]
	struct PlaceResp {
		location: Location,
	}

	let resp = reqwest::Client::new()
		.get(format!("https://places.googleapis.com/v1/places/{place_id}"))
		.header("X-Goog-Api-Key", key)
		.header("X-Goog-FieldMask", "location")
		.send()
		.await
		.map_err(|e| PlaceError::Other(e.to_string()))?;

	if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
		// Google sends `Retry-After` in integer seconds; 60s is a safe floor if it's missing.
		let retry_after_secs = resp
			.headers()
			.get(reqwest::header::RETRY_AFTER)
			.and_then(|v| v.to_str().ok())
			.and_then(|s| s.parse::<i64>().ok())
			.unwrap_or(60);
		return Err(PlaceError::RateLimited { retry_after_secs });
	}

	let resp = resp.error_for_status().map_err(|e| PlaceError::Other(e.to_string()))?;
	let body: PlaceResp = resp.json().await.map_err(|e| PlaceError::Other(e.to_string()))?;
	Ok(crate::domain::Coords {
		lat: body.location.latitude,
		lng: body.location.longitude,
	})
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	use super::*;

	// The seed-resolution invariant: a pre-seeds file (one bare arrangement) is adopted
	// as `default`, a seeds map resolves per band group (xl/lg, md, sm/xs) falling back
	// to `default`, legacy per-band keys fold into their group, and any other shape is
	// rejected loudly.
	#[test]
	fn seed_resolution() {
		use dockview_dioxus::Breakpoint::*;
		let resolve = |file: &str, bp: dockview_dioxus::Breakpoint| {
			let seeds = parse_seeds(file).unwrap();
			seeds.get(seed_key(bp)).or_else(|| seeds.get("default")).cloned()
		};
		assert_eq!(resolve(r#"{"version":1,"grid":{}}"#, Md), Some(serde_json::json!({"version":1,"grid":{}})));
		assert_eq!(resolve(r#"{"default":1,"md":2}"#, Md), Some(serde_json::json!(2)));
		assert_eq!(resolve(r#"{"default":1,"md":2}"#, Xl), Some(serde_json::json!(1)));
		assert_eq!(resolve(r#"{"md":2}"#, Xl), None);
		assert_eq!(resolve(r#"{"xl":1}"#, Lg), Some(serde_json::json!(1)));
		assert_eq!(resolve(r#"{"sm":1}"#, Xs), Some(serde_json::json!(1)));
		assert_eq!(resolve(r#"{"lg":2}"#, Xl), Some(serde_json::json!(2)));
		assert_eq!(resolve(r#"{"xl":1,"lg":2,"xs":3}"#, Lg), Some(serde_json::json!(1)));
		assert!(parse_seeds(r#"{"bogus":1}"#).is_err());
		assert!(parse_seeds("[]").is_err());
		assert!(parse_seeds(DEFAULT_LAYOUT).unwrap().contains_key("default"), "committed seeds must carry a default");
	}
}

#[cfg(not(target_arch = "wasm32"))]
async fn app_state() -> Result<AppState, ServerFnError> {
	use dioxus::{fullstack::FullstackContext, server::axum::Extension};
	let Extension(state) = FullstackContext::extract::<Extension<AppState>, _>().await?;
	Ok(state)
}

#[cfg(not(target_arch = "wasm32"))]
fn to_server_err(e: crate::error::DomainError) -> ServerFnError {
	ServerFnError::new(e.to_string())
}
