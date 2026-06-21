//! Throwaway: seed a temp DB and assert the portfolio + bundled pics land on disk.
use real_estate_allocation::store::{PropertyRepository, SqliteStore, seed};

#[tokio::main]
async fn main() {
	let dir = std::env::temp_dir().join("rea_seed_smoke");
	let _ = std::fs::remove_dir_all(&dir);
	let store = SqliteStore::open(&dir.join("app.db"), dir.join("properties")).await.expect("open store");
	seed(&store).await.expect("seed");

	let all = store.list(None).await.expect("list");
	assert_eq!(all.len(), 6, "expected 6 portfolio properties");
	let mut priced = 0;
	for p in &all {
		let files = store.list_files(p.id).await.expect("list files");
		assert!(!files.is_empty(), "{} has no pics", p.name);
		for f in &files {
			let path = store.file_path(p.id, f.id, &f.filename);
			let bytes = std::fs::read(&path).expect("pic on disk");
			assert!(bytes.len() > 1000, "{} pic suspiciously small", f.filename);
		}
		// Every property's developer must resolve to a developers row (FK contract).
		let dev = p.developer.as_ref().expect("seed sets a developer on every property");
		let resolved = store.get_developer(dev).await.expect("get developer");
		assert!(resolved.is_some(), "{} references unknown developer {dev}", p.name);
		if p.price.is_some() {
			priced += 1;
		}
		let price = p.price.map(|m| m.to_string()).unwrap_or_else(|| "?".into());
		println!("{:<42} {price:>9}  {:<18} {} pics  dev={dev}", p.name, p.construction.as_ref(), files.len());
	}
	assert_eq!(priced, 4, "the 4 built properties are priced; the 2 under-construction are not");
	println!("OK");
}
