// The entire producer: one `mfe!` invocation. It generates the custom-element
// registration, the `wasm-bindgen(start)` entrypoint, the origin self-derivation,
// the server-fn base override, and `MFE_MANIFEST` (emitted by the build as
// `mfe.json`). `embed::Overview` is the reused REA portfolio surface.
ev_lib::mfe! {
	service: "real-estate", name: "overview", kind: component,
	root: real_estate_allocation::embed::Overview, stylesheet: "mfe.css"
}
