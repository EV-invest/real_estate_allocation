#[cfg(not(target_arch = "wasm32"))]
fn main() {
	use std::{sync::Arc, time::Duration};

	use clap::Parser;
	use dioxus::LaunchBuilder;
	use real_estate_allocation::{
		App,
		config::{LiveSettings, SettingsFlags},
		store::{SqliteStore, seed},
	};

	#[derive(Default, Parser)]
	#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about, long_about = None)]
	struct Cli {
		#[command(flatten)]
		settings: SettingsFlags,
	}

	v_utils::clientside!();
	let cli = Cli::parse();
	let live_settings = match LiveSettings::new(cli.settings, Duration::from_secs(5)) {
		Ok(ls) => Arc::new(ls),
		Err(e) => {
			eprintln!("Error reading config: {e}");
			for cause in e.chain().skip(1) {
				eprintln!("  Caused by: {cause}");
			}
			return;
		}
	};
	let config = live_settings.config().expect("config valid on startup").clone();

	let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
	let store = rt.block_on(async {
		let store = SqliteStore::open(&config.db_path, config.data_dir.clone().into()).await.expect("open sqlite store");
		seed(&store).await.expect("seed sample properties");
		store
	});

	// Fullstack axum server. `with_context` makes both the store and config
	// available to every `#[server]` fn via `FromContext`.
	LaunchBuilder::server().with_context(store).with_context(config).launch(App);
}

#[cfg(target_arch = "wasm32")]
fn main() {
	dioxus::launch(real_estate_allocation::App);
}
