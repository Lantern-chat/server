
-- NOTE: Just search for `REFERENCES lantern.files` to find which tables should be here
CREATE OR REPLACE VIEW lantern.agg_used_files(id) AS
SELECT file_id FROM lantern.user_assets
UNION ALL
SELECT file_id FROM lantern.user_asset_files
UNION ALL
SELECT file_id FROM lantern.attachments
;
