//! Visual-parity aid for the iframe overview. Diffs our live `/embed/overview`
//! against the persisted landing "Premium Asset Portfolio" section
//! (`portfolio_original.html`) so we can see where our port drifts.
//!
//! NOT a pass/fail test. Exact HTML equality is the wrong oracle here:
//!   - our SSR render injects hydration markers (`data-node-hydration`,
//!     `<!--node-id-->`) the Next.js original never has,
//!   - our rsx emits the same Tailwind classes in a different token order,
//!   - the two property tiles sit behind an async `use_resource`, so the SSR
//!     snapshot shows `Skeleton`s, not cards — compare those by screenshot.
//! So we normalize away the noise (strip markers/comments, sort class tokens)
//! and print a `diff` of the *static chrome*. What's left is real drift to fix
//! by hand (e.g. `font-mono-tech` → `font-mono`, `font-serif-display` →
//! `font-serif`). Intentional differences (live data, renamed stats) stay.
//!
//! Run `dx serve` first; point at it with `OVERVIEW_URL`.
use std::{path::Path, process::Command};

const DEFAULT_URL: &str = "http://localhost:8080/embed/overview";

const VOID: &[&str] = &["meta", "link", "img", "br", "hr", "input", "source", "path", "circle", "rect", "line"];
/// The landing's design tokens vs ours — different *names*, identical CSS in our
/// build (`.font-serif` → Playfair, `.text-main-mist/40` ≡ `--muted-foreground`).
/// Canonicalize both sides so the diff ignores vocabulary and shows real drift.
const EQUIV: &[(&str, &str)] = &[
	("font-mono-tech", "font-mono"),
	("font-serif-display", "font-serif"),
	("text-muted-foreground", "text-main-mist/40"),
];
fn main() {
	let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
	let original = std::fs::read_to_string(dir.join("portfolio_original.html")).expect("persisted original section");

	let url = std::env::var("OVERVIEW_URL").unwrap_or_else(|_| DEFAULT_URL.into());
	let out = Command::new("curl").args(["-sf", "-m", "10", &url]).output().expect("spawn curl");
	assert!(out.status.success(), "fetch {url} failed — is `dx serve` up? set OVERVIEW_URL to override");
	let ours = String::from_utf8(out.stdout).expect("overview is utf-8");

	let orig = pretty(section(&strip_comments(&original)));
	let ours = pretty(section(&strip_comments(&ours)));

	let tmp = std::env::temp_dir();
	let (a, b) = (tmp.join("portfolio_original.pretty.html"), tmp.join("portfolio_ours.pretty.html"));
	std::fs::write(&a, &orig).unwrap();
	std::fs::write(&b, &ours).unwrap();
	// `diff` exits 1 on differences; we want its output regardless, so don't check status.
	Command::new("diff").arg("-u").arg(&a).arg(&b).status().expect("spawn diff");

	let differing = orig.lines().zip(ours.lines()).filter(|(a, b)| a != b).count();
	println!("\n{differing} differing chrome lines (ignoring hydration/class-order noise). Property tiles render as Skeletons over SSR — compare those by screenshot.");
}

/// Strip HTML comments — Dioxus SSR node markers (`<!--node-id5-->`, `<!--#-->`)
/// and React text separators (`<!-- -->`) are pure noise for this comparison.
fn strip_comments(html: &str) -> String {
	let mut s = html.to_string();
	while let Some(a) = s.find("<!--") {
		match s[a..].find("-->") {
			Some(b) => s.replace_range(a..a + b + 3, ""),
			None => break,
		}
	}
	s
}

/// The first `<section …>…</section>` (neither doc nests sections, so the first
/// `</section>` closes it).
fn section(html: &str) -> &str {
	let start = html.find("<section").expect("a <section> in the markup");
	let end = html[start..].find("</section>").expect("matching </section>") + start + "</section>".len();
	&html[start..end]
}

/// Tag-indent HTML, one node per line. Cosmetic — just makes the `diff` legible.
fn pretty(html: &str) -> String {
	let mut out = String::new();
	let mut depth: usize = 0;
	let mut rest = html.trim();
	while !rest.is_empty() {
		if let Some(lt) = rest.find('<') {
			let text = rest[..lt].trim();
			if !text.is_empty() {
				out.push_str(&"  ".repeat(depth));
				out.push_str(text);
				out.push('\n');
			}
			let gt = rest[lt..].find('>').expect("tag opened without closing >") + lt;
			let tag = &rest[lt..=gt];
			let closing = tag.starts_with("</");
			let void = tag.starts_with("<!") || tag.ends_with("/>") || VOID.contains(&tag_name(tag));
			if closing {
				depth = depth.saturating_sub(1);
			}
			out.push_str(&"  ".repeat(depth));
			out.push_str(&clean_tag(tag));
			out.push('\n');
			if !closing && !void {
				depth += 1;
			}
			rest = &rest[gt + 1..];
		} else {
			let text = rest.trim();
			if !text.is_empty() {
				out.push_str(&"  ".repeat(depth));
				out.push_str(text);
				out.push('\n');
			}
			break;
		}
	}
	out
}

fn tag_name(tag: &str) -> &str {
	tag.trim_start_matches('<').trim_start_matches('/').split([' ', '>', '/']).next().unwrap_or("")
}

/// Drop SSR/wrapper-only attributes, canonicalize token vocabulary, and sort
/// `class` tokens — so the diff ignores hydration markers, `data-slot` wrappers,
/// equivalent class names, and Tailwind ordering.
fn clean_tag(tag: &str) -> String {
	let mut t = drop_attr(tag, "data-node-hydration");
	t = drop_attr(&t, "data-slot");
	if let Some(s) = t.find("class=\"") {
		let val_start = s + "class=\"".len();
		let end = val_start + t[val_start..].find('"').expect("class value closes");
		let mut tokens: Vec<String> = t[val_start..end]
			.split_whitespace()
			.map(|tok| EQUIV.iter().find(|(from, _)| *from == tok).map_or(tok, |(_, to)| to).to_string())
			.collect();
		tokens.sort_unstable();
		t.replace_range(val_start..end, &tokens.join(" "));
	}
	t
}

fn drop_attr(tag: &str, attr: &str) -> String {
	let needle = format!(" {attr}=\"");
	match tag.find(&needle) {
		None => tag.to_string(),
		Some(a) => {
			let val_start = a + needle.len();
			let end = val_start + tag[val_start..].find('"').expect("attr value closes") + 1;
			let mut s = tag.to_string();
			s.replace_range(a..end, "");
			s
		}
	}
}
