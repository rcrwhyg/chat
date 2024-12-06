-- Add migration script here
-- create user table
CREATE TABLE IF NOT EXISTS users(
    id bigserial PRIMARY KEY,
    ws_id bigint NOT NULL,
    full_name varchar(64) NOT NULL,
    email varchar(64) NOT NULL,
    -- hashed argon2 password, length 97
    password_hash varchar(97) NOT NULL,
    created_at timestamptz DEFAULT CURRENT_TIMESTAMP
);

-- workspace for users
CREATE TABLE IF NOT EXISTS workspaces(
    id bigserial PRIMARY KEY,
    name varchar(32) UNIQUE,
    owner_id bigint NOT NULL REFERENCES users(id),
    created_at timestamptz DEFAULT CURRENT_TIMESTAMP
);

-- add super user 0 and workspace 0
BEGIN;
INSERT INTO users(id, ws_id, full_name, email, password_hash)
    VALUES (0, 0, 'Super User', 'super@user.com', '');
INSERT INTO workspaces(id, name, owner_id)
    VALUES (0, 'Default Workspace', 0);
COMMIT;

-- add foreign key constraint for ws_id in users table
ALTER TABLE users
    ADD CONSTRAINT users_ws_id_fk FOREIGN KEY (ws_id) REFERENCES workspaces(id);

-- create chat type: single, group, private_channel, public_channel
CREATE TYPE chat_type AS ENUM(
    'single',
    'group',
    'private_channel',
    'public_channel'
);

-- create chat table
CREATE TABLE IF NOT EXISTS chats(
    id bigserial PRIMARY KEY,
    ws_id bigint NOT NULL REFERENCES workspaces(id),
    name varchar(64),
    type chat_type NOT NULL,
    -- use id list
    members bigint[] NOT NULL,
    created_at timestamptz DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (ws_id, name, members)
);

-- create message table
CREATE TABLE IF NOT EXISTS messages(
    id bigserial PRIMARY KEY,
    chat_id bigint NOT NULL REFERENCES chats(id),
    sender_id bigint NOT NULL REFERENCES users(id),
    content text NOT NULL,
    files text[] NOT NULL DEFAULT '{}',
    created_at timestamptz DEFAULT CURRENT_TIMESTAMP
);

-- create index for messages for chat_id and created_at order by created_at desc
CREATE INDEX IF NOT EXISTS messages_chat_id_created_at_index ON messages(chat_id, created_at DESC);

-- create index for messages for sender_id and created_at order by created_at desc
CREATE INDEX IF NOT EXISTS messages_sender_id_created_at_index ON messages(sender_id, created_at DESC);

-- create index for chat members
CREATE INDEX IF NOT EXISTS chats_members_index ON chats USING GIN(members);
