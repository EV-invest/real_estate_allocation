//! R2 (S3-compatible) snapshot sync for the local DB + property files, driven by the
//! `db push`/`pull`/`status` CLI. The model is deliberately single-writer, last-push-wins:
//! one small versioned tarball plus a `manifest.json` pointer. Versioning exists so
//! divergence is *loud* — `push` refuses to overwrite a remote that advanced past your
//! last sync, `pull` refuses to clobber unpushed local changes, and `status` names the
//! state. There is no merge: this is a portfolio app with one operator, not a database.
//!
//! State is the three configured paths (`db_path`, `data_dir`, `layout_path`), captured
//! under fixed archive names so it restores regardless of where each lives. A content
//! hash over the sorted (name, bytes) set is the identity of a snapshot — deterministic,
//! independent of tar/mtime, so it doubles as a corruption check on download.

use std::{
	fs,
	io::{Read as _, Write as _},
	path::{Path, PathBuf},
	str::FromStr as _,
};

use object_store::{ObjectStore, PutPayload, aws::AmazonS3Builder, path::Path as ObjPath};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::{config::AppConfig, error::DomainError};

// Fixed archive entry names → restored to the configured paths on pull.
const DB_ENTRY: &str = "app.db";
const LAYOUT_ENTRY: &str = "dashboard_layout.json";
const DATA_PREFIX: &str = "properties/";

#[derive(Deserialize, Serialize)]
struct Manifest {
	version: u64,
	state_hash: String,
	created_at: i64,
	created_by: String,
}

#[derive(Deserialize, Serialize)]
struct Marker {
	version: u64,
	state_hash: String,
}

struct Targets {
	db: PathBuf,
	data: PathBuf,
	layout: PathBuf,
}

impl Targets {
	fn of(config: &AppConfig) -> Self {
		Self {
			db: config.db_path.clone().inner(),
			data: config.data_dir.clone().inner(),
			layout: config.layout_path.clone().inner(),
		}
	}
}

pub async fn push(config: &AppConfig, force: bool) -> Result<(), DomainError> {
	let t = Targets::of(config);
	let (entries, ch) = current(&t).await?;
	let (store, prefix) = client(config)?;
	let remote = fetch_manifest(store.as_ref(), &prefix).await?;
	let base = load_marker().map(|m| m.version).unwrap_or(0);

	if let Some(r) = &remote {
		if r.state_hash == ch {
			println!("already in sync at v{} ({})", r.version, short(&ch));
			return Ok(());
		}
		// Remote advanced past the version we last synced from: someone else pushed.
		// Overwriting would drop their snapshot — refuse unless explicitly forced.
		if r.version > base && !force {
			return Err(DomainError::Conflict(format!(
				"remote is at v{} but your last sync was v{base} — `db pull` first, or --force to overwrite",
				r.version
			)));
		}
	}

	let version = remote.as_ref().map(|r| r.version).unwrap_or(0) + 1;
	let tgz = make_tgz(&entries)?;
	store.put(&obj(&prefix, &format!("data-{version}.tgz")), PutPayload::from(tgz)).await.map_err(os_err)?;
	let manifest = Manifest {
		version,
		state_hash: ch.clone(),
		created_at: jiff::Timestamp::now().as_second(),
		created_by: host(),
	};
	let bytes = serde_json::to_vec_pretty(&manifest).expect("manifest is plain data, serializes");
	store.put(&obj(&prefix, "manifest.json"), PutPayload::from(bytes)).await.map_err(os_err)?;
	save_marker(&Marker { version, state_hash: ch.clone() })?;
	println!("pushed v{version} ({})", short(&ch));
	Ok(())
}

pub async fn pull(config: &AppConfig, force: bool) -> Result<(), DomainError> {
	let t = Targets::of(config);
	let (store, prefix) = client(config)?;
	let remote = fetch_manifest(store.as_ref(), &prefix).await?.ok_or_else(|| DomainError::NotFound {
		entity: "remote snapshot",
		id: prefix.clone(),
	})?;
	let (_, ch) = current(&t).await?;

	if remote.state_hash == ch {
		// Same bytes, just adopt the remote version so our marker stops looking stale.
		save_marker(&Marker {
			version: remote.version,
			state_hash: remote.state_hash.clone(),
		})?;
		println!("already up to date (v{})", remote.version);
		return Ok(());
	}
	if is_dirty(&ch) && !force {
		return Err(DomainError::Conflict("local has unpushed changes — `db push` first, or --force to discard them".into()));
	}

	let res = store.get(&obj(&prefix, &format!("data-{}.tgz", remote.version))).await.map_err(os_err)?;
	let bytes = res.bytes().await.map_err(os_err)?;
	let entries = extract(&bytes)?;
	let got = hash(&entries);
	if got != remote.state_hash {
		return Err(DomainError::Repository(format!(
			"downloaded snapshot hash {} != manifest {} — corrupt",
			short(&got),
			short(&remote.state_hash)
		)));
	}
	restore(&t, &entries)?;
	save_marker(&Marker {
		version: remote.version,
		state_hash: remote.state_hash.clone(),
	})?;
	println!("pulled v{} ({})", remote.version, short(&remote.state_hash));
	Ok(())
}

pub async fn status(config: &AppConfig) -> Result<(), DomainError> {
	let t = Targets::of(config);
	let (_, ch) = current(&t).await?;
	let (store, prefix) = client(config)?;
	let remote = fetch_manifest(store.as_ref(), &prefix).await?;
	let local = load_marker();

	println!("local data : {}", short(&ch));
	match &local {
		Some(m) => println!("last sync  : v{} ({})", m.version, short(&m.state_hash)),
		None => println!("last sync  : never"),
	}
	match &remote {
		Some(r) => println!("remote     : v{} ({}) by {}", r.version, short(&r.state_hash), r.created_by),
		None => println!("remote     : none"),
	}

	let dirty = is_dirty(&ch);
	let verdict = match &remote {
		None if ch == empty_hash() => "empty — nothing to sync",
		None => "local only — `db push` to publish",
		Some(r) => {
			let remote_ahead = r.version > local.as_ref().map(|m| m.version).unwrap_or(0);
			match (dirty, remote_ahead) {
				(false, false) => "in sync",
				(true, false) => "local ahead — `db push`",
				(false, true) => "remote ahead — `db pull`",
				(true, true) => "DIVERGED — local and remote both changed; reconcile manually (--force picks a side)",
			}
		}
	};
	println!("verdict    : {verdict}");
	Ok(())
}

/// Current on-disk state: `(entries, hash)`. Folds the WAL into the DB first so the
/// snapshot reflects committed writes even while a server holds the file open.
async fn current(t: &Targets) -> Result<(Vec<(String, Vec<u8>)>, String), DomainError> {
	checkpoint(&t.db).await?;
	let entries = collect(t)?;
	let h = hash(&entries);
	Ok((entries, h))
}

/// Local changed since the last sync (or has data but was never synced).
fn is_dirty(current_hash: &str) -> bool {
	match load_marker() {
		Some(m) => m.state_hash != current_hash,
		None => current_hash != empty_hash(),
	}
}

async fn checkpoint(db: &Path) -> Result<(), DomainError> {
	if !db.exists() {
		return Ok(());
	}
	let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db.display())).map_err(sqlx_err)?.create_if_missing(false);
	let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.map_err(sqlx_err)?;
	sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(&pool).await.map_err(sqlx_err)?;
	pool.close().await;
	Ok(())
}

fn collect(t: &Targets) -> Result<Vec<(String, Vec<u8>)>, DomainError> {
	let mut out = Vec::new();
	if t.db.exists() {
		out.push((DB_ENTRY.into(), fs::read(&t.db).map_err(io_err)?));
	}
	if t.layout.exists() {
		out.push((LAYOUT_ENTRY.into(), fs::read(&t.layout).map_err(io_err)?));
	}
	if t.data.exists() {
		walk(&t.data, &t.data, &mut out)?;
	}
	out.sort_by(|a, b| a.0.cmp(&b.0));
	Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), DomainError> {
	for entry in fs::read_dir(dir).map_err(io_err)? {
		let path = entry.map_err(io_err)?.path();
		if path.is_dir() {
			walk(root, &path, out)?;
		} else {
			let rel = path.strip_prefix(root).expect("walk descends only under root").to_string_lossy().replace('\\', "/");
			out.push((format!("{DATA_PREFIX}{rel}"), fs::read(&path).map_err(io_err)?));
		}
	}
	Ok(())
}

/// Wipe the target paths and rewrite them from the snapshot. Deletes propagate: a file
/// dropped from the remote is gone locally too, so the on-disk set matches the hash.
fn restore(t: &Targets, entries: &[(String, Vec<u8>)]) -> Result<(), DomainError> {
	for suffix in ["", "-wal", "-shm"] {
		// remove-if-present: a `-wal`/`-shm` need not exist, and the DB itself may be new.
		let _ = fs::remove_file(format!("{}{suffix}", t.db.display()));
	}
	if t.data.exists() {
		fs::remove_dir_all(&t.data).map_err(io_err)?;
	}
	// remove-if-present: layout is written only after the user saves one.
	let _ = fs::remove_file(&t.layout);

	for (name, bytes) in entries {
		let path = match name.as_str() {
			DB_ENTRY => t.db.clone(),
			LAYOUT_ENTRY => t.layout.clone(),
			n if n.starts_with(DATA_PREFIX) => t.data.join(&n[DATA_PREFIX.len()..]),
			other => return Err(DomainError::Repository(format!("unknown archive entry: {other}"))),
		};
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent).map_err(io_err)?;
		}
		fs::write(&path, bytes).map_err(io_err)?;
	}
	Ok(())
}

fn make_tgz(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, DomainError> {
	let mut tar_buf = Vec::new();
	{
		let mut ar = tar::Builder::new(&mut tar_buf);
		for (name, bytes) in entries {
			let mut header = tar::Header::new_gnu();
			header.set_size(bytes.len() as u64);
			header.set_mode(0o644);
			header.set_cksum();
			ar.append_data(&mut header, name, bytes.as_slice()).map_err(io_err)?;
		}
		ar.finish().map_err(io_err)?;
	}
	let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
	enc.write_all(&tar_buf).map_err(io_err)?;
	enc.finish().map_err(io_err)
}

fn extract(tgz: &[u8]) -> Result<Vec<(String, Vec<u8>)>, DomainError> {
	let mut ar = tar::Archive::new(flate2::read::GzDecoder::new(tgz));
	let mut out = Vec::new();
	for entry in ar.entries().map_err(io_err)? {
		let mut e = entry.map_err(io_err)?;
		let name = e.path().map_err(io_err)?.to_string_lossy().into_owned();
		let mut bytes = Vec::new();
		e.read_to_end(&mut bytes).map_err(io_err)?;
		out.push((name, bytes));
	}
	out.sort_by(|a, b| a.0.cmp(&b.0));
	Ok(out)
}

/// Content identity of a snapshot: sha256 over the sorted, length-delimited
/// (name, bytes) set. Order-independent and tar-independent, so it survives a
/// round-trip and flags a corrupt download.
fn hash(entries: &[(String, Vec<u8>)]) -> String {
	let mut h = Sha256::new();
	for (name, bytes) in entries {
		h.update((name.len() as u64).to_le_bytes());
		h.update(name.as_bytes());
		h.update((bytes.len() as u64).to_le_bytes());
		h.update(bytes);
	}
	h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn empty_hash() -> String {
	hash(&[])
}

fn short(hash: &str) -> &str {
	&hash[..hash.len().min(8)]
}

fn client(config: &AppConfig) -> Result<(Box<dyn ObjectStore>, String), DomainError> {
	if config.sync_bucket.is_empty() || config.sync_endpoint.is_empty() {
		return Err(DomainError::Validation("sync not configured: set sync_bucket and sync_endpoint".into()));
	}
	let key = env_secret("R2_ACCESS_KEY_ID")?;
	let secret = env_secret("R2_SECRET_ACCESS_KEY")?;
	let s3 = AmazonS3Builder::new()
		.with_bucket_name(&config.sync_bucket)
		.with_endpoint(&config.sync_endpoint)
		.with_access_key_id(key)
		.with_secret_access_key(secret)
		.with_region("auto") // R2 ignores region but the S3 client requires one.
		.with_virtual_hosted_style_request(false) // R2 is path-style: {endpoint}/{bucket}/{key}.
		.build()
		.map_err(|e| DomainError::Repository(format!("r2 client: {e}")))?;
	Ok((Box::new(s3), config.sync_prefix.clone()))
}

async fn fetch_manifest(store: &dyn ObjectStore, prefix: &str) -> Result<Option<Manifest>, DomainError> {
	match store.get(&obj(prefix, "manifest.json")).await {
		Ok(res) => {
			let bytes = res.bytes().await.map_err(os_err)?;
			Ok(Some(serde_json::from_slice(&bytes).map_err(|e| DomainError::Repository(format!("manifest parse: {e}")))?))
		}
		Err(object_store::Error::NotFound { .. }) => Ok(None),
		Err(e) => Err(os_err(e)),
	}
}

fn obj(prefix: &str, name: &str) -> ObjPath {
	ObjPath::from(format!("{prefix}/{name}"))
}

fn marker_path() -> PathBuf {
	PathBuf::from(format!("{}/{}/sync.json", v_utils::io::xdg::xdg_state_fallback(), env!("CARGO_PKG_NAME")))
}

fn load_marker() -> Option<Marker> {
	// A missing or unreadable marker is a valid "never synced here" state, handled by callers.
	fs::read(marker_path()).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

fn save_marker(m: &Marker) -> Result<(), DomainError> {
	let path = marker_path();
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent).map_err(io_err)?;
	}
	fs::write(&path, serde_json::to_vec_pretty(m).expect("marker is plain data, serializes")).map_err(io_err)
}

fn host() -> String {
	// Cosmetic provenance for the manifest; "unknown" is an acceptable label if unset.
	std::env::var("HOSTNAME")
		.ok()
		.filter(|s| !s.is_empty())
		.or_else(|| fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
		.unwrap_or_else(|| "unknown".into())
}

fn env_secret(var: &str) -> Result<String, DomainError> {
	std::env::var(var).map_err(|_| DomainError::Validation(format!("{var} not set in env")))
}

fn io_err(e: std::io::Error) -> DomainError {
	DomainError::Repository(format!("io: {e}"))
}

fn os_err(e: object_store::Error) -> DomainError {
	DomainError::Repository(format!("r2: {e}"))
}

fn sqlx_err(e: sqlx::Error) -> DomainError {
	DomainError::Repository(format!("sqlite: {e}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	// The data-loss invariant: archiving a state and extracting it must preserve the
	// content hash. If this breaks, `pull`'s corruption guard would reject good data
	// (or worse, accept bad). One round-trip covers make_tgz + extract + hash.
	#[test]
	fn tgz_roundtrip_preserves_hash() {
		let entries = vec![
			(DB_ENTRY.to_string(), b"sqlite-bytes".to_vec()),
			(format!("{DATA_PREFIX}a/1.jpg"), vec![0u8, 1, 2, 3]),
			(LAYOUT_ENTRY.to_string(), b"{}".to_vec()),
		];
		let before = hash(&entries);
		let round = extract(&make_tgz(&entries).unwrap()).unwrap();
		assert_eq!(before, hash(&round), "hash must survive a tar/gzip round-trip");
	}
}
