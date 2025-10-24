-- Add migration script here

-- organizations: Holds information about a publishing entity.
CREATE TABLE organizations (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE
);

-- organization_admins: Links users (by public key) to organizations they can manage.
CREATE TABLE organization_admins (
    org_id TEXT NOT NULL,
    pubkey_b64 TEXT NOT NULL,
    PRIMARY KEY (org_id, pubkey_b64),
    FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE CASCADE
);

-- games: Represents a specific game title belonging to an organization.
CREATE TABLE games (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE RESTRICT
);

-- game_versions: A specific, publishable version of a game.
CREATE TABLE game_versions (
    id TEXT PRIMARY KEY NOT NULL,
    game_id TEXT NOT NULL,
    version TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    publisher_pubkey_b64 TEXT NOT NULL,
    manifest_signature_b64 TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending_upload', 'uploading', 'published', 'failed', 'archived')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE (game_id, version),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- game_files: A content-addressable store of all unique files, identified by their hash.
CREATE TABLE game_files (
    sha256 TEXT PRIMARY KEY NOT NULL,
    size INTEGER NOT NULL,
    storage_path TEXT NOT NULL UNIQUE,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- version_file_links: A link table mapping files to specific game versions.
CREATE TABLE version_file_links (
    version_id TEXT NOT NULL,
    file_sha256 TEXT NOT NULL,
    -- The relative path of the file within the game's directory structure (e.g., "assets/player.png")
    file_path TEXT NOT NULL,
    PRIMARY KEY (version_id, file_path),
    FOREIGN KEY (version_id) REFERENCES game_versions(id) ON DELETE CASCADE,
    FOREIGN KEY (file_sha256) REFERENCES game_files(sha256) ON DELETE RESTRICT
);
