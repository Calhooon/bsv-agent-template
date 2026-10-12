CREATE TABLE IF NOT EXISTS payment_claims (
    nonce TEXT PRIMARY KEY,
    agent TEXT,
    created_at INTEGER
);
