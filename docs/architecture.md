# Storage and scan architecture (0.6)

`battles.db` contains the relational statistics and unfiltered replay JSON. `battles.archive.db` contains the original replay bytes packed losslessly into ZIP BLOBs. For a custom database name the archive is named `<stem>.archive.db` in the same directory. Always back up or move both files together; close the application first or use SQLite's online backup mechanism for a consistent snapshot. A missing archive is reported instead of silently creating an empty replacement.

The common battle table is `battles`, with typed columns such as `date`, `player`, `tank`, `role`, `damage`, `shots` and `hits`. Optional financial values are in `battle_economy`. Main statistics and player/damage views share this battle identity. Other canonical tables are `players`, `battle_participants`, `incoming_damage`, `outgoing_damage`, `replay_documents`, `projection_index`, `sources` and `app_meta`.

The German names (`gefechte`, `spieler`, `gefecht_teilnehmer`, `schaden_eingehend`, `schaden_ausgehend`, `replay_json`, `replay_sql_index`) are read-only compatibility views. The existing joined `v_*` views remain available. `battle_json` generates the old own-battle JSON representation on demand; it does not store a second summary. Old SQL that extracted values from `battles.payload` should use `battle_json.payload`, or preferably query the typed columns directly:

```sql
SELECT DISTINCT tank, tier, role
FROM battles
WHERE role GLOB '*_sniper'
ORDER BY tank;
```

`replay_archives` now contains only archive metadata. In the SQL browser the separate archive is attached read-only as `archive_store`; its BLOBs are available in `archive_store.replay_blobs`. All original bytes remain recoverable, including unknown JSON fields and the undecoded binary stream. Replay JSON remains additionally indexed in the statistics database for arbitrary SQL queries.

Schema updates live in `migrations.rs` and are versioned with `PRAGMA user_version`. Legacy BLOBs are copied, committed and byte-compared before the old BLOB table is removed. Conversion of summaries and relations is transactional, with a foreign-key check before commit. Invalid legacy JSON rolls back the schema migration. After successful separation, VACUUM releases the former BLOB pages. This first migration may take several minutes and temporarily needs additional disk space. Older binaries must not run against the migrated database.

The background `scan::Worker` retains its database connection. The first load builds typed battle records once; subsequent scans return only changed records. An unchanged scan never reloads or deserializes all battle summaries. Source fingerprints use file size, Unix modification time in nanoseconds, and parser version. Existing Windows `SystemTime` fingerprints are converted during migration. Source paths are normalized to avoid reimports caused solely by slash/case differences.

Vehicle catalogs are cached by canonical `scripts.pkg` path, size and modification time. Catalog metadata updates run only when the actual catalog contents change. The relational replay projection runs once per import, only for changed/unprocessed archives. Unchanged scans do not invalidate the player/damage views.

The UI is split into the application shell and dedicated modules for background work, preferences, settings/archive management, filters, overview, tanks, battles, financial statistics and trends. Damage/player search and replay details remain separate modules.

## Validation tools

```powershell
cargo test --offline
cargo run --release --offline --example scan_benchmark -- <test-database> <game-directory> <replay-directory>
```

Use a copy when benchmarking migration. Tests cover old schemas, byte-preserved archives, unknown fields, missing archives, rollback, catalog invalidation, incremental updates, localized placeholders, persisted language settings and all views at both supported window sizes in all 13 languages.
