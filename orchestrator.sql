DROP TABLE IF EXISTS bundles;

CREATE TABLE IF NOT EXISTS bundles (
    id INT AUTO_INCREMENT PRIMARY KEY,
    optimistic_hash VARCHAR(66),
    bundle_txid VARCHAR(66),
    data_size INT,
    is_settled BOOLEAN,
    content_type VARCHAR(255)
);

CREATE INDEX idx_bundles_id ON bundles(id);
CREATE INDEX idx_bundles_optimistic_hash ON bundles(optimistic_hash);
CREATE INDEX idx_bundles_bundle_txid ON bundles(bundle_txid);
CREATE INDEX idx_bundles_data_size ON bundles(data_size);
CREATE INDEX idx_bundles_is_settled ON bundles(is_settled);
CREATE INDEX idx_bundles_content_type ON bundles(content_type);
