use dioxus::prelude::*;

/// Self-hosted brand webfonts (Inter + Playfair Display), the families named by
/// the `--font-sans` / `--font-serif` / `--font-mono` chains in `assets/tokens.css`.
/// Render once near the app root: it injects the `@font-face` rules over the
/// bundled variable TTFs, so the dashboard gets brand typography with no CDN,
/// identically offline and behind a CSP.
///
/// This used to be `ev_lib::uikit::Fonts`. It left the kit in 0.10.0 (#105 —
/// brand assets are not a UI kit's business) and landed nowhere, so without a
/// copy here the token chains name "Inter" with nothing defining it and every
/// surface falls to `ui-sans-serif`. The four files are OFL, licences beside
/// them.
///
/// The landing supplies the same families through `next/font`; the embed
/// deliberately ships none — see the snapshot's note on system-font fallback.
#[component]
pub fn Fonts() -> Element {
	rsx! {
		document::Style { {format!(
			"@font-face{{font-family:'Inter';font-style:normal;font-weight:100 900;font-display:swap;src:url('{INTER}') format('truetype')}}\
			 @font-face{{font-family:'Inter';font-style:italic;font-weight:100 900;font-display:swap;src:url('{INTER_ITALIC}') format('truetype')}}\
			 @font-face{{font-family:'Playfair Display';font-style:normal;font-weight:300 900;font-display:swap;src:url('{PLAYFAIR}') format('truetype')}}\
			 @font-face{{font-family:'Playfair Display';font-style:italic;font-weight:300 900;font-display:swap;src:url('{PLAYFAIR_ITALIC}') format('truetype')}}",
			INTER = asset!("/assets/fonts/Inter.ttf"),
			INTER_ITALIC = asset!("/assets/fonts/Inter-Italic.ttf"),
			PLAYFAIR = asset!("/assets/fonts/PlayfairDisplay.ttf"),
			PLAYFAIR_ITALIC = asset!("/assets/fonts/PlayfairDisplay-Italic.ttf"),
		)} }
	}
}
