{
  admin_token = "trust_me_bro";
  admins = []; # currently inactive
  socket_addr = "127.0.0.1:62834";
  maps_api_key.env = "GOOGLE_MAPS_KEY";
  cors_allowed_origins = [ "http://localhost:58843" "http://localhost:58844" ];
  sync_bucket = "ev-invest-state";
  sync_endpoint = "https://1dbedc392b294bdef442b64e9030ba96.r2.cloudflarestorage.com";
  sync_prefix = "real-estate-allocation";
}
