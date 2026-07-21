-- initial schema
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY NOT NULL,
    profile_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(profile_id) REFERENCES profiles(id)
);

CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    base_url TEXT NOT NULL,
    secret_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
);

CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY NOT NULL,
    provider_id TEXT NOT NULL,
    model_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    capabilities_json TEXT NOT NULL,
    context_length INTEGER,
    FOREIGN KEY(provider_id) REFERENCES providers(id)
);

CREATE TABLE IF NOT EXISTS model_presets (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_id TEXT,
    name TEXT NOT NULL,
    model_name TEXT NOT NULL,
    capabilities_json TEXT NOT NULL,
    temperature REAL,
    system_prompt TEXT,
    in_random_pool INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY(provider_id) REFERENCES providers(id),
    FOREIGN KEY(model_id) REFERENCES models(id)
);

CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    title TEXT NOT NULL,
    mode TEXT NOT NULL,
    active_branch_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
);

CREATE TABLE IF NOT EXISTS conversation_settings (
    conversation_id TEXT PRIMARY KEY NOT NULL,
    mode TEXT NOT NULL,
    system_prompt TEXT,
    temperature REAL,
    model_preset_ids_json TEXT NOT NULL,
    candidate_pool_json TEXT NOT NULL,
    slot_count INTEGER NOT NULL,
    arena_kind TEXT,
    arena_category TEXT,
    max_concurrency INTEGER NOT NULL,
    image_size TEXT,
    image_aspect_ratio TEXT,
    FOREIGN KEY(conversation_id) REFERENCES conversations(id)
);

CREATE TABLE IF NOT EXISTS branches (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    parent_branch_id TEXT,
    fork_candidate_id TEXT,
    head_message_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(conversation_id) REFERENCES conversations(id)
);

CREATE TABLE IF NOT EXISTS message_nodes (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    parent_message_id TEXT,
    role TEXT NOT NULL,
    round_id TEXT,
    candidate_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(conversation_id) REFERENCES conversations(id),
    FOREIGN KEY(branch_id) REFERENCES branches(id)
);

CREATE TABLE IF NOT EXISTS content_blocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    block_json TEXT NOT NULL,
    FOREIGN KEY(message_id) REFERENCES message_nodes(id)
);

CREATE TABLE IF NOT EXISTS rounds (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    user_message_id TEXT NOT NULL,
    selected_candidate_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(conversation_id) REFERENCES conversations(id),
    FOREIGN KEY(branch_id) REFERENCES branches(id),
    FOREIGN KEY(user_message_id) REFERENCES message_nodes(id)
);

CREATE TABLE IF NOT EXISTS candidate_generations (
    id TEXT PRIMARY KEY NOT NULL,
    round_id TEXT NOT NULL,
    slot_label TEXT NOT NULL,
    model_preset_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_name TEXT NOT NULL,
    status TEXT NOT NULL,
    anonymous INTEGER NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY(round_id) REFERENCES rounds(id)
);

CREATE TABLE IF NOT EXISTS round_snapshots (
    round_id TEXT PRIMARY KEY NOT NULL,
    mode TEXT NOT NULL,
    system_prompt TEXT,
    temperature REAL,
    model_preset_ids_json TEXT NOT NULL,
    arena_kind TEXT,
    arena_category TEXT,
    image_size TEXT,
    image_aspect_ratio TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(round_id) REFERENCES rounds(id)
);

CREATE TABLE IF NOT EXISTS arena_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    category TEXT,
    candidate_pool_json TEXT NOT NULL,
    slot_count INTEGER NOT NULL,
    capability_requirements_json TEXT NOT NULL,
    allow_same_provider INTEGER NOT NULL,
    scoring_profile TEXT NOT NULL,
    reveal_policy TEXT NOT NULL,
    continuation_policy TEXT NOT NULL,
    FOREIGN KEY(conversation_id) REFERENCES conversations(id)
);

CREATE TABLE IF NOT EXISTS arena_votes (
    id TEXT PRIMARY KEY NOT NULL,
    round_id TEXT NOT NULL,
    vote_kind TEXT NOT NULL,
    selected_candidate_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(round_id) REFERENCES rounds(id)
);

CREATE TABLE IF NOT EXISTS ranking_events (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    arena_kind TEXT NOT NULL,
    category TEXT,
    winner_preset_id TEXT NOT NULL,
    loser_preset_id TEXT NOT NULL,
    round_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY(round_id) REFERENCES rounds(id)
);

CREATE TABLE IF NOT EXISTS analytics_events (
    id TEXT PRIMARY KEY NOT NULL,
    event_type TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    conversation_id TEXT,
    round_id TEXT,
    candidate_id TEXT,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
);

CREATE TABLE IF NOT EXISTS usage_records (
    candidate_id TEXT PRIMARY KEY NOT NULL,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    cost_usd REAL,
    latency_ms INTEGER,
    ttft_ms INTEGER,
    FOREIGN KEY(candidate_id) REFERENCES candidate_generations(id)
);

CREATE TABLE IF NOT EXISTS media_objects (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    mime TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id)
);
