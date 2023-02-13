use std::io::Write;

pub fn write_regex<W: Write>(name: &str, re: &str, mut out: W) -> Result<(), Box<dyn std::error::Error>> {
    use regex_automata::RegexBuilder;

    let re = RegexBuilder::new()
        .minimize(true)
        .ignore_whitespace(true)
        .unicode(true)
        .build_with_size::<u16>(re)?;

    let mut size = 16;
    let mut forward = re.forward().to_bytes_native_endian()?;
    let mut reverse = re.reverse().to_bytes_native_endian()?;

    // try to shrink to u8s if possible
    if let (Ok(f), Ok(r)) = (re.forward().to_u8(), re.reverse().to_u8()) {
        size = 8;
        forward = f.to_bytes_native_endian()?;
        reverse = r.to_bytes_native_endian()?;
    }

    write!(
        out, "pub static {1}: once_cell::sync::Lazy<Regex<DenseDFA<&'static [u{0}], u{0}>>> = once_cell::sync::Lazy::new(|| unsafe {{ Regex::from_dfas(DenseDFA::from_bytes(&{2:?}), DenseDFA::from_bytes(&{3:?})) }});",
        size, name, forward, reverse,
    )?;

    Ok(())
}
