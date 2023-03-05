use std::fmt::{self, Write};

pub fn format_list<I, T>(mut out: impl Write, list: impl IntoIterator<IntoIter = I>) -> Result<(), fmt::Error>
where
    I: Iterator<Item = T>,
    T: fmt::Display,
{
    let list = list.into_iter();
    let (len, _) = list.size_hint();

    for (idx, item) in list.enumerate() {
        let delim = match idx {
            _ if (idx + 1) == len => "",
            _ if len == 2 && idx == 0 => " and ",
            _ if (idx + 2) == len => ", and ",
            _ => ", ",
        };
        write!(out, "{item}{delim}")?;
    }

    Ok(())
}
