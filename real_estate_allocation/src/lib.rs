#![feature(default_field_values)]

pub mod api;
pub mod app;
pub mod dashboard;
pub mod domain;
// Only the microfrontend bundle renders the embed surface now; gating it off the
// dashboard build drops the `ev_lib::mfe` SDK (and its wasm-bindgen JS) from it.
#[cfg(feature = "mfe")]
pub mod embed;
pub mod error;
pub mod factors;
pub mod map;
pub mod panels;
mod uikit;

// Server-only: `config` pulls v_utils' xdg/io (native-gated) + filesystem
// `LiveSettings`; `store` is the persistence layer. Neither is linked into the
// wasm client, which reaches the server purely through `api`.
#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod store;

pub use app::App;
#[cfg(not(target_arch = "wasm32"))]
pub use ev_lib::architecture;
