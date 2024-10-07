-- Add migration script here
-- workspace for users
CREATE TABLE IF NOT EXISTS workspaces (
    id bigserial PRIMARY KEY,
    name varchar(32) NOT NULL UNIQUE,
    owner_id bigint NOT NULL REFERENCES users(id),
    created_at timestamptz DEFAULT CURRENT_TIMESTAMP
);

-- alter users table to add ws_id
ALTER TABLE
    users
ADD
    COLUMN ws_id bigint REFERENCES workspaces(id);

-- add super user 0 and workspace 0
BEGIN;

INSERT INTO
    users (id, full_name, email, password_hash)
VALUES
    (
        0,
        'Super User',
        'super@user.com',
        -- '$argon2id$v=19$m=65536,t=3,p=4$Z2F0ZXdheQ$Z+6J7Z1Z8Y9Z7Z6Z5Z4Z3Z2Z1Z0Z9Z8Z7Z6Z5Z4Z3Z2Z1'
        ''
    );

INSERT INTO
    workspaces(id, name, owner_id)
VALUES
    (0, 'Default Workspace', 0);

UPDATE
    users
SET
    ws_id = 0
WHERE
    id = 0;

COMMIT;

-- alter user table to make ws_id not null
ALTER TABLE
    users
ALTER COLUMN
    ws_id
SET
    NOT NULL;
