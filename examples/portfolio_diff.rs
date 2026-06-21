//! Visual-parity aid for the iframe overview. SSR-renders our `Overview`
//! component in-process and diffs it against the persisted landing "Premium
//! Asset Portfolio" section (`portfolio_original.html`) so we can see where our
//! port drifts. No running server needed — `cargo r --example portfolio_diff`.
//!
//! NOT a pass/fail test. Exact HTML equality is the wrong oracle here:
//!   - our rsx emits the same Tailwind classes in a different token order,
//!   - the landing uses token *names* (`font-mono-tech`, `font-serif-display`,
//!     `text-muted-foreground`) that compile to identical CSS as ours.
//! So we normalize the noise away (strip comments/`data-slot`, canonicalize the
//! equivalent class names, sort class tokens) and print a `diff` of what's left
//! — the real drift to fix by hand.
use std::{path::Path, process::Command};

use dioxus::prelude::*;
use real_estate_allocation::embed::Overview;

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
	let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
	let original = std::fs::read_to_string(dir.join("portfolio_original.html")).expect("persisted original section");

	let mut dom = VirtualDom::new(Overview);
	dom.rebuild_in_place();
	let ours = dioxus_ssr::render(&dom);

	// Dioxus SSR and Next.js encode the same entities differently (`&#38;` vs
	// `&amp;`, `&#39;` vs `&#x27;`); canonicalize both so the diff ignores it.
	let orig = decode_entities(&pretty(section(&strip_comments(&original))));
	let ours = decode_entities(&pretty(section(&strip_comments(&ours))));

	let tmp = std::env::temp_dir();
	let (a, b) = (tmp.join("portfolio_original.pretty.html"), tmp.join("portfolio_ours.pretty.html"));
	std::fs::write(&a, &orig).unwrap();
	std::fs::write(&b, &ours).unwrap();
	// `diff` exits 1 on differences; we want its output regardless, so don't check status.
	Command::new("diff").arg("-u").arg(&a).arg(&b).status().expect("spawn diff");

	let differing = orig.lines().zip(ours.lines()).filter(|(a, b)| a != b).count();
	println!("\n{differing} differing lines (ignoring class-order / equivalent-name noise).");
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

/// Collapse the entity forms the two SSR engines disagree on to raw characters.
fn decode_entities(s: &str) -> String {
	[
		("&#38;", "&"),
		("&amp;", "&"),
		("&#39;", "'"),
		("&#x27;", "'"),
		("&#62;", ">"),
		("&gt;", ">"),
		("&#60;", "<"),
		("&lt;", "<"),
		("&quot;", "\""),
	]
	.iter()
	.fold(s.to_string(), |acc, (from, to)| acc.replace(from, to))
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
