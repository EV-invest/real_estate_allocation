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
  # cors_allowed_origins = [ "https://<prod-landing-origin>" ];
  # Set once the landing origin is decided (docs/ARCHITECTURE.md defers it); until
  # then the build-time dev default (localhost only) applies.
}
