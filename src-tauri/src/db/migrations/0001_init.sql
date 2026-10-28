-- Initial schema for the invoice application (PRD §7).
-- Money is stored in integer minor units; quantities in integer thousandths.

CREATE TABLE IF NOT EXISTS products (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    name              TEXT    NOT NULL,
    normalized_name   TEXT    NOT NULL,
    sku               TEXT,
    base_price_minor  INTEGER NOT NULL CHECK (base_price_minor >= 0),
    deleted_at        TEXT,
    created_at        TEXT    NOT NULL,
    updated_at        TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_products_normalized_name ON products(normalized_name);
CREATE INDEX IF NOT EXISTS idx_products_sku            ON products(sku);
CREATE INDEX IF NOT EXISTS idx_products_alive           ON products(deleted_at) WHERE deleted_at IS NULL;

-- Full-text search over the normalized name. The `trigram` tokenizer gives
-- true substring matching, which is what partial Arabic search needs.
CREATE VIRTUAL TABLE IF NOT EXISTS products_fts USING fts5(
    name_norm,
    content='products',
    content_rowid='id',
    tokenize='trigram'
);

-- Keep the FTS index in sync with the products table.
CREATE TRIGGER IF NOT EXISTS products_fts_insert AFTER INSERT ON products BEGIN
    INSERT INTO products_fts(rowid, name_norm) VALUES (new.id, new.normalized_name);
END;

CREATE TRIGGER IF NOT EXISTS products_fts_delete AFTER DELETE ON products BEGIN
    INSERT INTO products_fts(products_fts, rowid, name_norm) VALUES ('delete', old.id, old.normalized_name);
END;

CREATE TRIGGER IF NOT EXISTS products_fts_update AFTER UPDATE ON products BEGIN
    INSERT INTO products_fts(products_fts, rowid, name_norm) VALUES ('delete', old.id, old.normalized_name);
    INSERT INTO products_fts(rowid, name_norm) VALUES (new.id, new.normalized_name);
END;

CREATE TABLE IF NOT EXISTS invoices (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_number      TEXT UNIQUE,
    invoice_date        TEXT    NOT NULL,
    customer_name       TEXT    NOT NULL,
    customer_phone      TEXT,
    notes               TEXT,
    status              TEXT    NOT NULL CHECK (status IN ('draft', 'issued')),
    seller_snapshot_json TEXT   NOT NULL,
    total_minor         INTEGER NOT NULL DEFAULT 0,
    deleted_at          TEXT,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_invoices_created   ON invoices(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_invoices_customer  ON invoices(customer_name);
CREATE INDEX IF NOT EXISTS idx_invoices_number    ON invoices(invoice_number);
CREATE INDEX IF NOT EXISTS idx_invoices_status    ON invoices(status);
CREATE INDEX IF NOT EXISTS idx_invoices_deleted   ON invoices(deleted_at);

CREATE TABLE IF NOT EXISTS invoice_items (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id           INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    product_id           INTEGER REFERENCES products(id) ON DELETE SET NULL,
    name_snapshot        TEXT    NOT NULL,
    quantity_thousandths INTEGER NOT NULL CHECK (quantity_thousandths > 0),
    unit_price_minor     INTEGER NOT NULL CHECK (unit_price_minor >= 0),
    line_total_minor     INTEGER NOT NULL CHECK (line_total_minor >= 0),
    sort_order           INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_invoice_items_invoice ON invoice_items(invoice_id, sort_order);

CREATE TABLE IF NOT EXISTS app_settings (
    key        TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Durable invoice-number counter. Never decremented, never recycled.
CREATE TABLE IF NOT EXISTS invoice_number_sequence (
    name  TEXT PRIMARY KEY,
    value INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO invoice_number_sequence(name, value) VALUES ('invoices', 0);

CREATE TABLE IF NOT EXISTS drafts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    payload_json TEXT    NOT NULL,
    updated_at   TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS backup_history (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    destination       TEXT    NOT NULL,
    file_path         TEXT    NOT NULL,
    sha256            TEXT,
    size_bytes        INTEGER,
    result            TEXT    NOT NULL,
    created_at        TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS update_preferences (
    key        TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
