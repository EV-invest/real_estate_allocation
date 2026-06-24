
export function __ev_register(tag, mount){
  if (customElements.get(tag)) return;
  customElements.define(tag, class extends HTMLElement {
    connectedCallback(){ mount(this); }
  });
}
export function __ev_origin(){ return new URL(import.meta.url).origin; }
