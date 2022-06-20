use sdk::models::Snowflake;

pub fn process_embeds(msg_id: Snowflake, msg: &str) {
    let urls = embed_parser::msg::find_urls(msg);

    if urls.is_empty() {
        return; // TODO
    }
}
