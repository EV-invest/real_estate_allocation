
export function __ev_register(tag, mount){
  if (customElements.get(tag)) return;
  customElements.define(tag, class extends HTMLElement {
    connectedCallback(){ mount(this); }
  });
}
export function __ev_origin(){ return new URL(import.meta.url).origin; }
export function __ev_seed_hydration(){
  // No SSR here, so the globals the server normally injects are absent. dioxus-web's
  // forced hydration path reads both before any render: `initial_dioxus_hydration_data`
  // (atob'd → an empty CBOR array, no resolved server data) and `hydrate_queue` (the
  // streaming chunk queue it drains — empty, so the drain is a no-op).
  if (window.initial_dioxus_hydration_data === undefined) {
    window.initial_dioxus_hydration_data = "gA==";
  }
  if (window.hydrate_queue === undefined) {
    window.hydrate_queue = [];
  }
}
