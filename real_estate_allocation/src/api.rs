use dioxus::prelude::*;

use crate::domain::{FileKind, Property, PropertyFile, PropertyId, PropertyState};

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
pub async fn list_properties(filter: Option<Vec<PropertyState>>) -> Result<Vec<Property>, ServerFnError> {
	use ev::architecture::Specification;

	use crate::{domain::InState, store::PropertyRepository};

	let store = app_state().await?.store;

	// Or-combine the requested states; default = Purchased. The `Fn(&T)->bool`
	// blanket impl makes the disjunction itself a `Specification`.
	let states = filter.unwrap_or_else(|| vec![PropertyState::Purchased]);
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
		// Deterministic mock series, seeded from the id so it is stable per property.
		let seed = id.raw().as_u64_pair().0;
		p.price_series = v_utils::distributions::laplace_random_walk(100.0, 1000, 0.1, 0.0, Some(seed));
	}
	Ok(prop)
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
