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

	// First-boot bootstrap: a volume that has never synced (no marker — a fresh PVC,
	// or one predating R2 sync) adopts the latest remote snapshot before serving. A
	// volume that has synced is left to the operator (`db push`/`pull`). Fatal on
	// failure: booting on absent/stale data would silently serve the wrong thing.
	{
		use real_estate_allocation::sync;
		exit_on_error(rt.block_on(sync::bootstrap(&config)));
	}

	// Content otherwise arrives via `db pull`, never fabricated on boot — the server
	// only ensures the schema is current (via `open`) and serves what's there.
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
		http::{Method, header},
	};
	use tower_http::cors::{Any, CorsLayer};
	let app_state = real_estate_allocation::api::AppState { store, config };
	dioxus::server::serve(move || {
		let app_state = app_state.clone();
		async move {
			// Any origin may embed us: the `/api/embed` GET is public read-only and the
			// server-fn POSTs are token-authed (no ambient cookies), so `*` grants a
			// browser nothing a server-side client couldn't already fetch — and we stay
			// agnostic to whoever hosts the bundle. No `allow_credentials`, so `*` holds.
			let cors = CorsLayer::new()
				.allow_origin(Any)
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
	// dioxus 0.7.9's `launch` wires the server-fn root as `"" + "/" + base_path` —
	// a relative URL whose parse panics inside the first server-fn call
	// (RelativeUrlWithoutBase), killing hydration. Compose the absolute root
	// ourselves and go straight to the web launcher, skipping that block; the
	// wasm transport only ever sends `url.path()`, so the origin is parse ballast.
	let origin = web_sys::window().expect("wasm runs in a browser").location().origin().expect("origin is always readable");
	let base = dioxus::cli_config::base_path().map(|b| format!("/{}", b.trim_matches('/'))).unwrap_or_default();
	dioxus::fullstack::set_server_url(format!("{origin}{base}").leak());
	dioxus::web::launch::launch(real_estate_allocation::App, Vec::new(), Vec::new());
}
