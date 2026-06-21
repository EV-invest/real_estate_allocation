use dioxus::prelude::*;

use crate::domain::{FileKind, Property, PropertyFile, PropertyId, PropertyStateKind};

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

//HACK: see `main` — `LaunchBuilder::with_context` doesn't reach server fns in
// dioxus-server 0.7.9, so we read our state from the request extension instead.
// `FullstackContext` is the one handle available in both the SSR render path and
// the server-fn POST path.
#[server]
pub async fn list_properties(filter: Option<Vec<PropertyStateKind>>) -> Result<Vec<Property>, ServerFnError> {
	use ev_lib::architecture::Specification;

	use crate::{domain::InState, store::PropertyRepository};

	let store = app_state().await?.store;

	// Or-combine the requested states; default = Purchased. The `Fn(&T)->bool`
	// blanket impl makes the disjunction itself a `Specification`.
	let states = filter.unwrap_or_else(|| vec![PropertyStateKind::Purchased]);
	let spec = move |p: &Property| states.iter().any(|s| InState(*s).holds(p));
	let props = store.list(Some(&spec)).await.map_err(to_server_err)?;
	Ok(props)
}
#[server]
pub async fn get_property(id: PropertyId) -> Result<Option<Property>, ServerFnError> {
	use crate::store::PropertyRepository;

	let store = app_state().await?.store;
	let mut prop = store.get(id).await.map_err(to_server_err)?;
	if let Some(p) = prop.as_mut() {
		p.price_series = mock_series(id, p.state);
	}
	Ok(prop)
}

#[server]
pub async fn get_developer(name: String) -> Result<Option<crate::domain::Developer>, ServerFnError> {
	use crate::store::PropertyRepository;

	let store = app_state().await?.store;
	store.get_developer(&name).await.map_err(to_server_err)
}
#[server]
pub async fn list_files(id: PropertyId) -> Result<Vec<PropertyFile>, ServerFnError> {
	use crate::store::PropertyRepository;

	let store = app_state().await?.store;
	store.list_files(id).await.map_err(to_server_err)
}
#[server]
pub async fn upload_file(property_id: PropertyId, kind: FileKind, filename: String, content_type: String, bytes: Vec<u8>, token: String) -> Result<PropertyFile, ServerFnError> {
	use secrecy::ExposeSecret as _;

	use crate::store::PropertyRepository;

	let AppState { store, config: cfg } = app_state().await?;
	if token != cfg.admin_token.expose_secret() {
		return Err(ServerFnError::new("not authorized to upload"));
	}

	let file_id = crate::domain::FileId::new();
	let path = store.file_path(property_id, file_id, &filename);
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent).map_err(|e| ServerFnError::new(format!("create dir: {e}")))?;
	}
	std::fs::write(&path, &bytes).map_err(|e| ServerFnError::new(format!("write file: {e}")))?;

	let file = PropertyFile {
		id: file_id,
		property_id,
		kind,
		filename,
		content_type,
	};
	store.add_file(file.clone()).await.map_err(to_server_err)?;
	Ok(file)
}
#[server]
pub async fn file_bytes(id: PropertyId, file_id: crate::domain::FileId, filename: String) -> Result<Vec<u8>, ServerFnError> {
	let store = app_state().await?.store;
	let path = store.file_path(id, file_id, &filename);
	std::fs::read(&path).map_err(|e| ServerFnError::new(format!("read file: {e}")))
}
#[server]
pub async fn am_i_admin(token: String) -> Result<bool, ServerFnError> {
	use secrecy::ExposeSecret as _;

	let cfg = app_state().await?.config;
	Ok(!token.is_empty() && token == cfg.admin_token.expose_secret())
}
#[server]
pub async fn maps_api_key() -> Result<String, ServerFnError> {
	use secrecy::ExposeSecret as _;

	let cfg = app_state().await?.config;
	Ok(cfg.maps_api_key.expose_secret().to_string())
}
/// Deterministic mock value series, seeded from the id so it is stable per property.
/// Anchored to the purchase instant (a few weeks of pre-purchase tracking, then a
/// fixed run of weekly estimates clipped to now). A long-ago purchase therefore
/// produces a series that ends well before today — the chart fills that tail with a
/// dotted projection. Non-purchased properties anchor to a trailing window.
#[cfg(not(target_arch = "wasm32"))]
fn mock_series(id: PropertyId, state: crate::domain::PropertyState) -> Vec<(jiff::Timestamp, f64)> {
	use crate::domain::PropertyState;

	const WEEK: i64 = 7 * 24 * 3600;
	const LEAD: i64 = 8; // weeks of pre-purchase estimate tracking (drawn dimmed)
	const SPAN: usize = 30; // weeks of estimates generated

	let now = jiff::Timestamp::now();
	let anchor = match state {
		PropertyState::Purchased(ts) => ts,
		_ => jiff::Timestamp::from_second(now.as_second() - (SPAN as i64) * WEEK).expect("trailing window in range"),
	};
	let start = anchor.as_second() - LEAD * WEEK;

	let seed = id.raw().as_u64_pair().0;
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
