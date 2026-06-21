# Installation

```sh
nix develop      # toolchain + deps
dx serve         # dev server (fullstack)
```

## Configuration

Config is loaded via `LiveSettings` (see `examples/config.nix` for the shape).
Defaults put the SQLite DB at `./data/app.db` and uploaded/seed files under
`./data/properties`.

| key             | notes                                                                   |
|-----------------|-------------------------------------------------------------------------|
| `maps_api_key`  | Google Maps key. `examples/config.nix` reads it from `$GOOGLE_MAPS_KEY`. |
| `admin_token`   | gates file uploads (`upload_file`).                                      |
| `db_path`       | SQLite file; created + seeded on first run.                             |
| `data_dir`      | on-disk property files (pics/docs).                                      |
| `socket_addr`   | prod bind address (ignored under `dx serve`).                           |

### Google Maps key — required scope

Map pins are stored as **Google Place IDs** and resolved in-browser via the
**Places API (New)** (`Place.fetchFields(['location'])`); the loader requests
`libraries=places&v=weekly`. So the key must have **Places API (New) enabled**.
The legacy Places/Geocoding REST APIs are *not* used (and need billing) — don't
rely on them. If pins stop dropping, check that scope first.

## Seeding

The DB is seeded once, on first run against an **empty** `properties` table
(`store::seed`) — there is no runtime "add property" path yet. A non-empty DB is
left untouched, and there are **no migrations**: after a schema change, delete
`./data/app.db` to re-seed.
