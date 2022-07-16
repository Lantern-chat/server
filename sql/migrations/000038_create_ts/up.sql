-- CREATE TYPE lantern.languages AS ENUM (
--     'english',
--     'arabic',
--     'armenian',
--     'basque',
--     'catalan',
--     'danish',
--     'dutch',
--     'finnish',
--     'french',
--     'german',
--     'greek',
--     'hindi',
--     'hungarian',
--     'indonesian',
--     'irish',
--     'italian',
--     'lithuanian',
--     'nepali',
--     'norwegian',
--     'portuguese',
--     'romanian',
--     'russian',
--     'serbian',
--     'simple',
--     'spanish',
--     'swedish',
--     'tamil',
--     'turkish',
--     'yiddish',
-- );

-- THIS MUST MATCH `LanguageCode` in schema crate
CREATE OR REPLACE FUNCTION lantern.to_language(int2)
RETURNS regconfig
AS
$$
    SELECT CASE WHEN $1 = 0 THEN 'english'::regconfig
                WHEN $1 = 1 THEN 'arabic'::regconfig
                WHEN $1 = 2 THEN 'armenian'::regconfig
                WHEN $1 = 3 THEN 'basque'::regconfig
                WHEN $1 = 4 THEN 'catalan'::regconfig
                WHEN $1 = 5 THEN 'danish'::regconfig
                WHEN $1 = 6 THEN 'dutch'::regconfig
                WHEN $1 = 7 THEN 'finnish'::regconfig
                WHEN $1 = 8 THEN 'french'::regconfig
                WHEN $1 = 9 THEN 'german'::regconfig
                WHEN $1 = 10 THEN 'greek'::regconfig
                WHEN $1 = 11 THEN 'hindi'::regconfig
                WHEN $1 = 12 THEN 'hungarian'::regconfig
                WHEN $1 = 13 THEN 'indonesian'::regconfig
                WHEN $1 = 14 THEN 'irish'::regconfig
                WHEN $1 = 15 THEN 'italian'::regconfig
                WHEN $1 = 16 THEN 'lithuanian'::regconfig
                WHEN $1 = 17 THEN 'nepali'::regconfig
                WHEN $1 = 18 THEN 'norwegian'::regconfig
                WHEN $1 = 19 THEN 'portuguese'::regconfig
                WHEN $1 = 20 THEN 'romanian'::regconfig
                WHEN $1 = 21 THEN 'russian'::regconfig
                WHEN $1 = 22 THEN 'serbian'::regconfig
                WHEN $1 = 23 THEN 'simple'::regconfig
                WHEN $1 = 24 THEN 'spanish'::regconfig
                WHEN $1 = 25 THEN 'swedish'::regconfig
                WHEN $1 = 26 THEN 'tamil'::regconfig
                WHEN $1 = 27 THEN 'turkish'::regconfig
                WHEN $1 = 28 THEN 'yiddish'::regconfig
            ELSE 'english'::regconfig
        END
$$ LANGUAGE SQL IMMUTABLE;

ALTER TABLE lantern.messages ADD COLUMN IF NOT EXISTS ts tsvector
    -- take the top 6 bits of the smallint flags as a language code
    GENERATED ALWAYS AS (to_tsvector(lantern.to_language(flags >> 10), content)) STORED;

CREATE INDEX msg_ts_idx IF NOT EXISTS ON lantern.messages USING GIN (ts);