use std::error::Error;

use lettre::*;

use lettre::message::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let sender = "Admin <admin@lantern.chat>".parse()?;

    let m = Message::builder()
        .from(sender)
        .to("Nova <fpnova@pm.me>".parse()?)
        .subject("Email Testing")
        .body("Testing email".to_owned())?;

    Ok(())
}
