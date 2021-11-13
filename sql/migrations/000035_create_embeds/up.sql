CREATE TABLE IF NOT EXISTS lantern.embed_cache (
    id      bigint  NOT NULL,
    url     text    NOT NULL
);
ALTER TABLE lantern.embed_cache OWNER TO postgres;

