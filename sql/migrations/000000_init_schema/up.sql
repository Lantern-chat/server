-- Database itself is created manually in-code before the first migration is run

CREATE SCHEMA lantern;
ALTER SCHEMA lantern OWNER TO postgres;

ALTER SYSTEM SET enable_seqscan = 1;
ALTER SYSTEM SET jit = 0; -- honestly buggy, and we never create insane queries that need it anyway
ALTER SYSTEM SET random_page_cost = 1; -- Database on SSD
SELECT pg_reload_conf();

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

CREATE DOMAIN lantern.uint2 AS int4
   CHECK(VALUE >= 0 AND VALUE < 65536);