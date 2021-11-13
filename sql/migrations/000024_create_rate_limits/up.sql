CREATE TABLE lantern.rate_limits (
    violations  integer     NOT NULL DEFAULT 0,
    addr        inet        NOT NULL
);
ALTER TABLE lantern.rate_limits OWNER TO postgres;

CREATE INDEX rate_limit_idx ON lantern.rate_limits USING hash(addr);

CREATE TABLE lantern.ip_bans (
    expires     timestamp,
    address     inet,
    network     cidr
);
ALTER TABLE lantern.ip_bans OWNER TO postgres;

CREATE INDEX ip_bans_address_idx ON lantern.ip_bans USING btree(address) WHERE address IS NOT NULL;
CREATE INDEX ip_bans_network_idx ON lantern.ip_bans USING GIST(network inet_ops) WHERE network IS NOT NULL;

ALTER TABLE lantern.ip_bans ADD CONSTRAINT addr_check CHECK (
    address IS NOT NULL OR network IS NOT NULL
);