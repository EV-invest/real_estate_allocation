
let __reaMap = null;
let __reaMarkers = {};
let __reaFitted = false;

window.__reaMapsReady = function () { window.__reaMapsLoaded = true; };

function __reaColor(apartments) {
  // A building's pin colour is its strongest portfolio relationship across lots:
  // Purchased > Purchasing > Interesting. A lot's `Purchased` serialises as
  // `{Purchased: <ts>}`; the other statuses as plain strings.
  let purchasing = false, interesting = false;
  for (const a of (apartments || [])) {
    const kind = (typeof a.status === 'string') ? a.status : Object.keys(a.status)[0];
    if (kind === 'Purchased') return '#2e9e5b';
    if (kind === 'Purchasing') purchasing = true;
    else if (kind === 'Interesting') interesting = true;
  }
  if (purchasing) return '#e58aae';
  if (interesting) return '#f2c94c';
  return '#e6e1d3';
}

export function rea_render_markers(elId, propsJson, selectedId) {
  if (!window.google || !window.google.maps) { setTimeout(() => rea_render_markers(elId, propsJson, selectedId), 300); return; }
  const el = document.getElementById(elId);
  if (!el) return;
  const props = JSON.parse(propsJson);
  if (!__reaMap) {
    __reaMap = new google.maps.Map(el, { center: { lat: 13.78, lng: 109.22 }, zoom: 12, disableDefaultUI: true, backgroundColor: '#070d18' });
  }
  const seen = {};
  const bounds = new google.maps.LatLngBounds();
  props.forEach(p => {
    // Coords are resolved + cached server-side; an unresolved place just has no pin.
    if (!p.coords) return;
    const id = p.id;
    seen[id] = true;
    const pos = { lat: p.coords.lat, lng: p.coords.lng };
    const color = __reaColor(p.apartments);
    const scale = id === selectedId ? 11 : 7;
    const icon = { path: google.maps.SymbolPath.CIRCLE, fillColor: color, fillOpacity: 1, strokeColor: '#070d18', strokeWeight: 2, scale: scale };
    bounds.extend(pos);
    let m = __reaMarkers[id];
    if (!m) {
      m = new google.maps.Marker({ position: pos, map: __reaMap, icon: icon });
      m.addListener('click', () => { if (window.__reaSelect) window.__reaSelect(id); });
      __reaMarkers[id] = m;
    } else {
      m.setPosition(pos); m.setIcon(icon); m.setMap(__reaMap);
    }
  });
  Object.keys(__reaMarkers).forEach(id => { if (!seen[id]) { __reaMarkers[id].setMap(null); delete __reaMarkers[id]; } });
  // Fit once, so later selection re-renders don't yank the viewport around.
  if (!__reaFitted && !bounds.isEmpty()) { __reaMap.fitBounds(bounds, 48); __reaFitted = true; }
}

export function rea_on_select(cb) { window.__reaSelect = cb; }

export function rea_on_keynav(cb) {
  window.addEventListener('keydown', (e) => {
    if (e.key === 'ArrowLeft') cb(-1);
    else if (e.key === 'ArrowRight') cb(1);
  });
}

export function rea_sync_url(buildingId, appt) {
  const url = new URL(window.location.href);
  if (buildingId) { url.searchParams.set('building', buildingId); }
  else { url.searchParams.delete('building'); }
  if (appt) { url.searchParams.set('appt', appt); }
  else { url.searchParams.delete('appt'); }
  window.history.replaceState({}, '', url);
}

export function rea_url_building() {
  return new URL(window.location.href).searchParams.get('building') || '';
}

export function rea_url_appt() {
  return new URL(window.location.href).searchParams.get('appt') || '';
}

export function rea_sync_selection(csv) {
  const url = new URL(window.location.href);
  if (csv) { url.searchParams.set('selection', csv); }
  else { url.searchParams.delete('selection'); }
  window.history.replaceState({}, '', url);
}

export function rea_url_selection() {
  return new URL(window.location.href).searchParams.get('selection') || '';
}
