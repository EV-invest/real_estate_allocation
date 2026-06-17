#[cfg(not(target_arch = "wasm32"))]
fn main() {
	use std::{sync::Arc, time::Duration};

	use clap::Parser;
	use dioxus::LaunchBuilder;
	use real_estate_allocation::{
		App,
		config::{AppConfig, LiveSettings, SettingsCommand, SettingsFlags},
		store::{SqliteStore, seed},
	};
	use v_utils::utils::exit_on_error;

	#[derive(Parser)]
	#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about, long_about = None)]
	struct Cli {
		#[command(flatten)]
		settings: SettingsFlags,
		#[command(subcommand)]
		command: Option<Command>,
	}

	#[derive(clap::Subcommand)]
	enum Command {
		/// Manage configuration: write defaults, diff against defaults, and generate the JSON Schema / Nix module.
		Config {
			#[command(subcommand)]
			cmd: SettingsCommand,
		},
	}

	v_utils::clientside!();
	let cli = Cli::parse();
	if let Some(Command::Config { cmd }) = cli.command {
		// Never returns — performs the requested config operation and exits.
		AppConfig::handle_settings_command(cmd, cli.settings);
	}
	let live_settings = Arc::new(exit_on_error(LiveSettings::new(cli.settings, Duration::from_secs(5))));
	let config = live_settings.config().expect("config valid on startup").clone();

	let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
	let store = rt.block_on(async {
		let store = SqliteStore::open(config.db_path.as_ref(), config.data_dir.clone().inner()).await.expect("open sqlite store");
		seed(&store).await.expect("seed sample properties");
		store
	});

	// dioxus' server launch reads the bind address from these env vars
	// (`dioxus_cli_config::fullstack_address_or_localhost`), falling back to
	// 127.0.0.1:8080. Setting them from config is the only override it exposes.
	unsafe {
		std::env::set_var("IP", config.socket_addr.ip().to_string());
		std::env::set_var("PORT", config.socket_addr.port().to_string());
	}

	// Fullstack axum server. `with_context` makes both the store and config
	// available to every `#[server]` fn via `FromContext`.
	LaunchBuilder::server().with_context(store).with_context(config).launch(App);
}

#[cfg(target_arch = "wasm32")]
fn main() {
	dioxus::launch(real_estate_allocation::App);
}
