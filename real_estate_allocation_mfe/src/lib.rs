// The entire producer: one `mfe!` invocation. It generates the custom-element
// registration, the `wasm-bindgen(start)` entrypoint, the origin self-derivation,
// and `MFE_MANIFEST` (emitted by the build as `mfe.json`). `Overview` is the
// reused REA portfolio surface.
ev_lib::mfe! {
	service: "real-estate", name: "overview", kind: component,
	root: real_estate_allocation_embeds::Overview, stylesheet: "mfe.css"
}
