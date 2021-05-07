#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_query() {
        let query = Query::select()
            .columns([DirectMessage::UserA, DirectMessage::UserB].iter().copied())
            .from(DirectMessage::Table)
            .to_owned();

        let s = query.build(PostgresQueryBuilder).0;

        println!("{}", s);
    }
}
