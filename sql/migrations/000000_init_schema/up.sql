-- Database itself is created manually in-code before the first migration is run

CREATE SCHEMA lantern;
ALTER SCHEMA lantern OWNER TO postgres;

-- host table tracks migrations
CREATE TABLE lantern.host (
    migration int NOT NULL,
    migrated  timestamp NOT NULL,

    CONSTRAINT migration_primary_key PRIMARY KEY (migration)
);
ALTER TABLE lantern.host OWNER TO postgres;

CREATE OR REPLACE FUNCTION lantern.array_diff(lhs anyarray, rhs anyarray)
    RETURNS anyarray
    LANGUAGE sql immutable
AS $$
    SELECT COALESCE(array_agg(elem), '{}')
    FROM UNNEST(lhs) elem
    WHERE elem <> ALL(rhs)
$$;

CREATE OR REPLACE FUNCTION lantern.array_uniq(arr anyarray)
    RETURNS anyarray
    LANGUAGE sql immutable
AS $$
    SELECT ARRAY( SELECT DISTINCT UNNEST(arr) )
$$;