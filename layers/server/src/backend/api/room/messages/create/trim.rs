use std::borrow::Cow;

use crate::{Error, ServerState};

pub fn trim_message<'a>(state: &ServerState, content: &'a str) -> Result<Cow<'a, str>, Error> {
    let mut trimmed_content = Cow::Borrowed(content);

    if !trimmed_content.is_empty() {
        let mut trimming = false;
        let mut new_content = String::new();
        let mut count = 0;

        // TODO: Don't strip newlines inside code blocks?
        for (idx, &byte) in trimmed_content.as_bytes().iter().enumerate() {
            // if we encounted a newline
            if byte == b'\n' {
                count += 1; // count up

                // if over 2 consecutive newlines, begin trimming
                if count > 2 {
                    // if not already trimming, push everything tested up until this point
                    // notably not including the third newline
                    if !trimming {
                        trimming = true;
                        new_content.push_str(&trimmed_content[..idx]);
                    }

                    // skip any additional newlines
                    continue;
                }
            } else {
                // reset count if newline streak broken
                count = 0;
            }

            // if trimming, push the new byte
            if trimming {
                unsafe { new_content.as_mut_vec().push(byte) };
            }
        }

        if trimming {
            trimmed_content = Cow::Owned(new_content);
        }

        let newlines = bytecount::count(trimmed_content.as_bytes(), b'\n');

        let too_large = !state.config.message.message_len.contains(&trimmed_content.len());
        let too_long = newlines > state.config.message.max_newlines;

        if too_large || too_long {
            return Err(Error::BadRequest);
        }
    }

    Ok(trimmed_content)
}
