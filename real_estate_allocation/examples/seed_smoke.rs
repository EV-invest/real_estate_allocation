//! Throwaway: seed a temp DB and assert the portfolio + bundled pics land on disk.
use real_estate_allocation::store::{PropertyRepository, SqliteStore, seed};

#[tokio::main]
async fn main() {
	let dir = std::env::temp_dir().join("rea_seed_smoke");
	let _ = std::fs::remove_dir_all(&dir);
	let store = SqliteStore::open(&dir.join("app.db"), dir.join("properties")).await.expect("open store");
	seed(&store).await.expect("seed");

	let all = store.list(None).await.expect("list");
	assert_eq!(all.len(), 4, "expected 4 portfolio properties");
	for p in &all {
		let files = store.list_files(p.id).await.expect("list files");
		assert!(!files.is_empty(), "{} has no pics", p.name);
		for f in &files {
			let path = store.file_path(p.id, f.id, &f.filename);
			let bytes = std::fs::read(&path).expect("pic on disk");
			assert!(bytes.len() > 1000, "{} pic suspiciously small", f.filename);
		}
		println!("{:<42} {:>9}  {} pics  {}", p.name, p.price.to_string(), files.len(), p.research_url.as_str());
	}
	println!("OK");
}
