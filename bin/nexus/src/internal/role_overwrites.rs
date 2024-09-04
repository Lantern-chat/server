use sdk::models::*;

#[derive(Default)]
pub struct RawOverwrites {
    pub id: Vec<Snowflake>,
    pub a1: Vec<i64>,
    pub a2: Vec<i64>,
    pub d1: Vec<i64>,
    pub d2: Vec<i64>,
}

impl RawOverwrites {
    pub fn new(mut ows: ThinVec<Overwrite>) -> Self {
        if ows.len() > 1 {
            ows.sort_unstable_by_key(|ow| ow.id);
            ows.dedup_by_key(|ow| ow.id);
        }

        let mut raw = RawOverwrites::default();

        // collect overwrites in a SoA format that can be sent to the db
        for ow in ows {
            // ignore pointless overwrites
            if ow.allow.is_empty() && ow.deny.is_empty() {
                continue;
            }

            let [a1, a2] = ow.allow.to_i64();
            let [d1, d2] = ow.deny.to_i64();

            raw.id.push(ow.id);
            raw.a1.push(a1);
            raw.a2.push(a2);
            raw.d1.push(d1);
            raw.d2.push(d2);
        }

        raw
    }
}
