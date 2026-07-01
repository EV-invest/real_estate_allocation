# Prod config (secret-free, committable). Evaluated to JSON at image-build time
# by the flake — the runtime container carries no `nix`, only the baked result.
# The two secrets resolve at startup from the container env that gitops' k8s
# Secret injects (`envFrom` `real_estate_allocation-env`); `.env` is the config
# primitive's indirection, and a missing var fails the boot loudly. Everything
# else is the fixed in-container layout under the `/data` PVC.
{
  socket_addr = "0.0.0.0:59079";
  db_path = "/data/app.db";
  data_dir = "/data/properties";
  layout_path = "/data/dashboard_layout.json";
  admins = [ ];
  maps_api_key.env = "GOOGLE_MAPS_KEY";
  admin_token.env = "REA_ADMIN_TOKEN";
  # R2 snapshot sync (`db push`/`pull`). Bucket + endpoint are non-secret (the
  # account id in the endpoint is a public identifier); the access key / secret
  # come from the container env (k8s Secret `real_estate_allocation-env`):
  # R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY. With these set, a fresh `/data` PVC
  # auto-pulls the latest snapshot on first boot (see `main`).
  sync_bucket = "ev-invest-state";
  sync_endpoint = "https://1dbedc392b294bdef442b64e9030ba96.r2.cloudflarestorage.com";
  # The landing host (apex + www) fetches /api/embed cross-origin from this
  # server; without these it falls back to the localhost-only build default and
  # the browser blocks the embed's data fetch.
  cors_allowed_origins = [ "https://evinvest.ltd" "https://www.evinvest.ltd" ];
}
