CREATE TABLE IF NOT EXISTS lantern.metrics (
    ts      timestamp   NOT NULL DEFAULT now(),

    -- allocated memory usage, in bytes
    mem     bigint      NOT NULL,

    -- bytes uploaded by users since last metric
    upload  bigint      NOT NULL,

    -- number of messages since last metric
    msgs    int      NOT NULL,
    -- requests since last metric
    reqs    int      NOT NULL,
    -- errors since last metric
    errs    int      NOT NULL,
    -- number of connected gateway users
    conns   int      NOT NULL,

    -- latency percentiles
    p50     smallint    NOT NULL,
    p95     smallint    NOT NULL,
    p99     smallint    NOT NULL,
);
ALTER TABLE lantern.metrics OWNER TO postgres;

CREATE INDEX metrics_ts_idx ON lantern.metrics USING btree(ts);