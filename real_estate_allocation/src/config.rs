use std::net::SocketAddr;

use secrecy::SecretString;
use smart_default::SmartDefault;
use v_utils::{io::ExpandedPath, macros as v_macros};

#[derive(Clone, Debug, v_macros::LiveSettings, v_macros::MyConfigPrimitives, v_macros::Settings, SmartDefault)]
pub struct AppConfig {
	pub maps_api_key: SecretString,
	#[default(app_data("app.db"))]
	pub db_path: ExpandedPath,
	#[default(app_data("properties"))]
	pub data_dir: ExpandedPath,
	#[default(app_data("dashboard_layout.json"))]
	pub layout_path: ExpandedPath,
	pub admin_token: SecretString,
	#[serde(default)]
	pub admins: Vec<String>,
	/// Address the fullstack server binds to. Overrides dioxus' `127.0.0.1:8080` default.
	#[default(SocketAddr::from(([127, 0, 0, 1], 59079)))]
	pub socket_addr: SocketAddr,
	/// Origins allowed to call this server cross-origin. The landing host serves the
	/// microfrontend bundle from its own origin, so only the bundle's server-fn POSTs
	/// and the `/api/embed` GET reach this server cross-origin and need CORS. Dev
	/// default derives from the build-time `PORT` (see [`default_cors_origins`]); add
	/// prod origins via config.
	#[default(default_cors_origins())]
	pub cors_allowed_origins: Vec<String>,
	/// R2 (S3-compatible) bucket for `db push`/`pull` snapshots; empty disables sync.
	#[serde(default)]
	pub sync_bucket: String,
	/// R2 S3 API endpoint, e.g. `https://<accountid>.r2.cloudflarestorage.com`.
	#[serde(default)]
	pub sync_endpoint: String,
	/// Key prefix within the bucket, so one bucket can hold several apps' snapshots.
	#[default("real-estate-allocation".to_string())]
	pub sync_prefix: String,
}

/// Mutable state lives outside the repo, under `$XDG_DATA_HOME/real_estate_allocation/`
/// so a checkout never carries a DB. Prod overrides these to the `/data` PVC via
/// `deploy/config.nix`; real content arrives through `db pull`, not the repo.
fn app_data(file: &str) -> ExpandedPath {
	ExpandedPath::from(format!("{}/{}/{file}", v_utils::io::xdg::xdg_data_fallback(), env!("CARGO_PKG_NAME")))
}

/// Build-time-derived dev CORS allowlist: the landing page (`PORT`) and its API
/// origin (`PORT + 1`). `PORT` is baked by the flake; absent it (a bare `cargo
/// build`), we fall back to landing's `next dev` port. A *set* but unparseable
/// `PORT` is a build misconfig — panic rather than silently pick the fallback.
fn default_cors_origins() -> Vec<String> {
	let port: u16 = match option_env!("PORT") {
		Some(p) => p.parse().expect("PORT (build-time env) must be a valid u16"),
		None => 58843,
	};
	vec![format!("http://localhost:{port}"), format!("http://localhost:{}", port + 1)]
}
