use std::net::SocketAddr;

use secrecy::SecretString;
use smart_default::SmartDefault;
use v_utils::{io::ExpandedPath, macros as v_macros};

#[derive(Clone, Debug, v_macros::LiveSettings, v_macros::MyConfigPrimitives, v_macros::Settings, SmartDefault)]
pub struct AppConfig {
	pub maps_api_key: SecretString,
	#[default(ExpandedPath::from("./data/app.db"))]
	pub db_path: ExpandedPath,
	#[default(ExpandedPath::from("./data/properties"))]
	pub data_dir: ExpandedPath,
	pub admin_token: SecretString,
	#[serde(default)]
	pub admins: Vec<String>,
	/// Address the fullstack server binds to. Overrides dioxus' `127.0.0.1:8080` default.
	#[default(SocketAddr::from(([127, 0, 0, 1], 8080)))]
	pub socket_addr: SocketAddr,
}
