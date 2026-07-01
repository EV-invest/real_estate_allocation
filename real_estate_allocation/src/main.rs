#[cfg(not(target_arch = "wasm32"))]
fn main() {
	use std::{sync::Arc, time::Duration};

	use clap::Parser;
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
		/// Manage the database: apply migrations, seed the test fixture, and sync to/from R2.
		Db {
			#[command(subcommand)]
			cmd: DbCommand,
		},
	}

	#[derive(clap::Subcommand)]
	enum DbCommand {
		/// Apply any pending schema migrations.
		Migrate,
		/// Insert the sample portfolio (test fixture; no-op if the DB is non-empty).
		Seed,
		/// Delete the local DB + files, re-migrate, and reseed. Dev only.
		Reset,
		/// Snapshot the local DB + files to R2 as a new version.
		Push {
			/// Overwrite even if the remote advanced past your last sync.
			#[arg(long)]
			force: bool,
		},
		/// Replace the local DB + files with the latest R2 version.
		Pull {
			/// Discard local changes that were never pushed.
			#[arg(long)]
			force: bool,
		},
		/// Show local vs remote versions and whether they diverged.
		Status,
	}

	v_utils::clientside!();
	let Cli { settings, command } = Cli::parse();
	// `Config` runs before the config file is loaded (it may write a fresh one) and
	// never returns; only a `Db` command survives to be handled after config load.
	let db_command = match command {
		Some(Command::Config { cmd }) => {
			AppConfig::handle_settings_command(cmd, settings);
			return;
		}
		Some(Command::Db { cmd }) => Some(cmd),
		None => None,
	};

	let live_settings = Arc::new(exit_on_error(LiveSettings::new(settings, Duration::from_secs(5))));
	let config = live_settings.config().expect("config valid on startup").clone();

	let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

	if let Some(cmd) = db_command {
		use real_estate_allocation::sync;
		rt.block_on(async {
			let open = || SqliteStore::open(config.db_path.as_ref(), config.data_dir.clone().inner());
			let res = match cmd {
				// `open` applies migrations; a bare open is the migrate step.
				DbCommand::Migrate => open().await.map(|_| ()),
				DbCommand::Seed => match open().await {
					Ok(store) => seed(&store).await,
					Err(e) => Err(e),
				},
				DbCommand::Reset => {
					let db = config.db_path.as_ref();
					for suffix in ["", "-wal", "-shm"] {
						// remove-if-present: a clean reset need not have prior files.
						let _ = std::fs::remove_file(format!("{}{suffix}", db.display()));
					}
					let _ = std::fs::remove_dir_all(config.data_dir.clone().inner());
					let _ = std::fs::remove_file(config.layout_path.as_ref());
					match open().await {
						Ok(store) => seed(&store).await,
						Err(e) => Err(e),
					}
				}
				DbCommand::Push { force } => sync::push(&config, force).await,
				DbCommand::Pull { force } => sync::pull(&config, force).await,
				DbCommand::Status => sync::status(&config).await,
			};
			exit_on_error(res);
		});
		return;
	}

	// Real content arrives via `db pull`, never fabricated on boot — the server only
	// ensures the schema is current (via `open`) and serves what's there.
	let store = rt.block_on(async { SqliteStore::open(config.db_path.as_ref(), config.data_dir.clone().inner()).await.expect("open sqlite store") });

	// dioxus' server launch reads the bind address from these env vars
	// (`dioxus_cli_config::fullstack_address_or_localhost`), falling back to
	// 127.0.0.1:8080. Setting them from config is the only override it exposes.
	// Under `dx serve` the CLI owns the address (it sets these to its proxy
	// target and fronts us on its devserver), so we only override for prod.
	if std::env::var_os("DIOXUS_DEVSERVER_PORT").is_none() {
		unsafe {
			std::env::set_var("IP", config.socket_addr.ip().to_string());
			std::env::set_var("PORT", config.socket_addr.port().to_string());
		}
	}

	//HACK: dioxus-server 0.7.9 does not forward `LaunchBuilder::with_context`
	// to server functions — those contexts only reach the SSR vdom, so a
	// `consume_context` inside a `#[server]` fn panics ("Must be called from
	// inside a Dioxus runtime"). We instead attach the shared state as an axum
	// request extension, which is present on both the SSR render request and
	// every server-fn POST; `crate::api` reads it via `FullstackContext`.
	use dioxus::server::{
		axum::{Extension, Router},
		http::{HeaderValue, Method, header},
	};
	use tower_http::cors::CorsLayer;
	let app_state = real_estate_allocation::api::AppState { store, config };
	dioxus::server::serve(move || {
		let app_state = app_state.clone();
		async move {
			// The bundle runs on the landing page, so its server-fn POSTs and the
			// `/api/embed` GET are cross-origin from landing — one CORS layer over
			// the whole router covers them. (The bundle's own static assets are
			// served by the landing host, not here.)
			let origins = app_state
				.config
				.cors_allowed_origins
				.iter()
				.map(|o| o.parse::<HeaderValue>().expect("cors_allowed_origins entry is a valid Origin header value"))
				.collect::<Vec<_>>();
			let cors = CorsLayer::new()
				.allow_origin(origins)
				.allow_methods([Method::GET, Method::POST])
				.allow_headers([header::CONTENT_TYPE]);
			let router = Router::new()
				.merge(dioxus::server::router(App))
				.route("/health", dioxus::server::axum::routing::get(|| async { "ok" }))
				.route("/api/embed/building/{id}", dioxus::server::axum::routing::get(real_estate_allocation::api::building_json))
				.layer(Extension(app_state))
				.layer(cors);
			Ok(router)
		}
	});
}

#[cfg(target_arch = "wasm32")]
fn main() {
	dioxus::launch(real_estate_allocation::App);
}
