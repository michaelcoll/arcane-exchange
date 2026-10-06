CREATE TABLE card_import
(
    id                UUID PRIMARY KEY,
    user_id           VARCHAR(50) NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status            TEXT        NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed')),
    source_lines      INT         NOT NULL DEFAULT 0,
    total_lines       INT         NOT NULL DEFAULT 0,
    processed_lines   INT         NOT NULL DEFAULT 0,
    line_errors       JSONB       NOT NULL DEFAULT '[]'::jsonb,
    line_error_count  INT         NOT NULL DEFAULT 0,
    error_message     TEXT        NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at       TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX card_import_one_active_per_user ON card_import (user_id) WHERE status IN ('pending', 'running');

CREATE INDEX card_import_user_created_at ON card_import (user_id, created_at DESC);
