-- Database itself is created manually in-code before the first migration is run

CREATE SCHEMA lantern;
ALTER SCHEMA lantern OWNER TO postgres;

-- host table tracks migrations
CREATE TABLE lantern.host (
    migration int NOT NULL,
    migrated  timestamp NOT NULL DEFAULT,

    CONSTRAINT migration_primary_key PRIMARY KEY (migration)
);
ALTER TABLE lantern.host OWNER TO postgres;