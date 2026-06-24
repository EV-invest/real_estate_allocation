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
	#[default(ExpandedPath::from("./mfe-dist"))]
	pub mfe_dir: ExpandedPath,
	/// Origins allowed to call this server cross-origin. The landing host loads the
	/// microfrontend bundle into its own page, so the bundle's server-fn POSTs and
	/// the module/wasm/`/mfe` asset fetches all originate from the *landing* origin
	/// and need CORS. Dev default is landing's `npm run dev`; add prod origins here.
	#[default(vec!["http://localhost:3000".to_string()])]
	pub cors_allowed_origins: Vec<String>,
}
