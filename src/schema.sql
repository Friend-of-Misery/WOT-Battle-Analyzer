
    CREATE TABLE IF NOT EXISTS replay_documents (replay_path TEXT NOT NULL, block_index INTEGER NOT NULL, payload TEXT NOT NULL, PRIMARY KEY(replay_path,block_index));
    CREATE TABLE IF NOT EXISTS players (account_id INTEGER PRIMARY KEY, real_name TEXT);
    CREATE INDEX IF NOT EXISTS players_name ON players(real_name COLLATE NOCASE);
    CREATE TABLE IF NOT EXISTS battle_participants (battle_id TEXT NOT NULL REFERENCES battles(battle_id), vehicle_id INTEGER NOT NULL, player_id INTEGER REFERENCES players(account_id), display_name TEXT, real_name TEXT, alias TEXT, tank TEXT, tier INTEGER, team INTEGER,total_damage INTEGER,shots INTEGER,hits INTEGER,penetrations INTEGER,spotting_assist INTEGER,tracking_assist INTEGER,stun_assist INTEGER, PRIMARY KEY(battle_id,vehicle_id));
    CREATE INDEX IF NOT EXISTS participants_player ON battle_participants(player_id,battle_id);
    CREATE INDEX IF NOT EXISTS participants_tank ON battle_participants(tank);
    CREATE INDEX IF NOT EXISTS participants_alias ON battle_participants(alias COLLATE NOCASE);
    CREATE TABLE IF NOT EXISTS incoming_damage (battle_id TEXT NOT NULL, own_vehicle TEXT NOT NULL, vehicle_id INTEGER NOT NULL, life_index INTEGER NOT NULL, damage INTEGER, direct_hits INTEGER, penetrating_hits INTEGER, explosion_hits INTEGER, ammunition_type TEXT, blocked_damage INTEGER, killed_by INTEGER, PRIMARY KEY(battle_id,own_vehicle,vehicle_id,life_index), FOREIGN KEY(battle_id,vehicle_id) REFERENCES battle_participants(battle_id,vehicle_id));
    CREATE TABLE IF NOT EXISTS outgoing_damage (battle_id TEXT NOT NULL, own_vehicle TEXT NOT NULL, vehicle_id INTEGER NOT NULL, life_index INTEGER NOT NULL, damage INTEGER, direct_hits INTEGER, penetrating_hits INTEGER, explosion_hits INTEGER, ammunition_type TEXT, PRIMARY KEY(battle_id,own_vehicle,vehicle_id,life_index), FOREIGN KEY(battle_id,vehicle_id) REFERENCES battle_participants(battle_id,vehicle_id));
    CREATE TABLE IF NOT EXISTS projection_index (replay_path TEXT PRIMARY KEY, captured_at TEXT NOT NULL, version INTEGER NOT NULL);
    
CREATE TABLE IF NOT EXISTS app_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS sources(path TEXT PRIMARY KEY,size INTEGER NOT NULL,modified_ns INTEGER,parser_version INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS replay_archives(path TEXT PRIMARY KEY,original_size INTEGER NOT NULL,captured_at TEXT NOT NULL,format_version INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS replay_archive_battles(path TEXT PRIMARY KEY,identity TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS replay_archive_identity ON replay_archive_battles(identity);
