CREATE TABLE IF NOT EXISTS e2e_test_items (
    id serial PRIMARY KEY,
    name text NOT NULL,
    CONSTRAINT e2e_test_items_name_key UNIQUE (name)
);

CREATE TABLE IF NOT EXISTS e2e_test_tags (
    id serial PRIMARY KEY,
    label text NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS e2e_test_tags_label_idx ON e2e_test_tags (label);
