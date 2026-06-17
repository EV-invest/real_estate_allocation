use v_utils::macros as v_macros;

#[derive(Clone, Debug, Default, v_macros::LiveSettings, v_macros::MyConfigPrimitives, v_macros::Settings)]
pub struct AppConfig {
	#[primitives(skip)]
	#[serde(default = "__default_maps_api_key")]
	pub maps_api_key: String,
	#[primitives(skip)]
	#[serde(default = "__default_db_path")]
	pub db_path: String,
	#[primitives(skip)]
	#[serde(default = "__default_data_dir")]
	pub data_dir: String,
	#[primitives(skip)]
	#[serde(default)]
	pub admin_token: String,
	#[primitives(skip)]
	#[serde(default)]
	pub admins: Vec<String>,
}

fn __default_maps_api_key() -> String {
	String::new()
}
fn __default_db_path() -> String {
	"./data/app.db".to_string()
}
fn __default_data_dir() -> String {
	"./data/properties".to_string()
}
