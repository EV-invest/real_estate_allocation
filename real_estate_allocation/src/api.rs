use dioxus::prelude::*;

use crate::domain::{FileKind, Property, PropertyFile, PropertyId, PropertyState};

/// The only client↔server seam. Each `#[server]` fn runs on the host, pulling the
/// `SqliteStore` / `AppConfig` out of the fullstack server context (provided via
/// `LaunchBuilder::with_context` in `main`), and is called as an async fn from the
/// wasm client.

#[server]
pub async fn list_properties(filter: Option<Vec<PropertyState>>) -> Result<Vec<Property>, ServerFnError> {
	use ev::architecture::Specification;

	use crate::{domain::InState, store::PropertyRepository};

	let store: crate::store::SqliteStore = consume_context();

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

	let store: crate::store::SqliteStore = consume_context();
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

	let store: crate::store::SqliteStore = consume_context();
	store.list_files(id).await.map_err(to_server_err)
}

#[server]
pub async fn upload_file(property_id: PropertyId, kind: FileKind, filename: String, content_type: String, bytes: Vec<u8>, token: String) -> Result<PropertyFile, ServerFnError> {
	use secrecy::ExposeSecret as _;

	use crate::store::PropertyRepository;

	let store: crate::store::SqliteStore = consume_context();
	let cfg: crate::config::AppConfig = consume_context();
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
	let store: crate::store::SqliteStore = consume_context();
	let path = store.file_path(id, file_id, &filename);
	std::fs::read(&path).map_err(|e| ServerFnError::new(format!("read file: {e}")))
}

#[server]
pub async fn am_i_admin(token: String) -> Result<bool, ServerFnError> {
	use secrecy::ExposeSecret as _;

	let cfg: crate::config::AppConfig = consume_context();
	Ok(!token.is_empty() && token == cfg.admin_token.expose_secret())
}

#[server]
pub async fn maps_api_key() -> Result<String, ServerFnError> {
	use secrecy::ExposeSecret as _;

	let cfg: crate::config::AppConfig = consume_context();
	Ok(cfg.maps_api_key.expose_secret().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn to_server_err(e: crate::error::DomainError) -> ServerFnError {
	ServerFnError::new(e.to_string())
}
