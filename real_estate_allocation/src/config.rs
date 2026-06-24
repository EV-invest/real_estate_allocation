use std::net::SocketAddr;

use secrecy::SecretString;
use smart_default::SmartDefault;
use v_utils::{io::ExpandedPath, macros as v_macros};

#[derive(Clone, Debug, v_macros::LiveSettings, v_macros::MyConfigPrimitives, v_macros::Settings, SmartDefault)]
pub struct AppConfig {
	pub maps_api_key: SecretString,
	#[default(ExpandedPath::from("./public/data/app.db"))]
	pub db_path: ExpandedPath,
	#[default(ExpandedPath::from("./public/data/properties"))]
	pub data_dir: ExpandedPath,
	#[default(ExpandedPath::from("./public/dashboard_layout.json"))]
	pub layout_path: ExpandedPath,
	pub admin_token: SecretString,
	#[serde(default)]
	pub admins: Vec<String>,
	/// Address the fullstack server binds to. Overrides dioxus' `127.0.0.1:8080` default.
	#[default(SocketAddr::from(([127, 0, 0, 1], 59079)))]
	pub socket_addr: SocketAddr,
	/// Built microfrontend bundle (`nix run .#mfe` output), served at `/mfe`. The
	/// `embed::Overview` bundle derives its asset/server-fn URLs from this origin.
	#[default(ExpandedPath::from("./target/mfe-dist"))]
	pub mfe_dir: ExpandedPath,
	/// Origins allowed to call this server cross-origin. The landing host loads the
	/// microfrontend bundle into its own page, so the bundle's server-fn POSTs and
	/// the module/wasm/`/mfe` asset fetches all carry the *landing page's* origin and
	/// need CORS. Dev default derives from the build-time `PORT` (see
	/// [`default_cors_origins`]); add prod origins via config.
	#[default(default_cors_origins())]
	pub cors_allowed_origins: Vec<String>,
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
