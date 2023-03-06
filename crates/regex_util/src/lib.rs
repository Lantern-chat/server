pub mod rt {
    #[repr(align(1))]
    pub struct DenseDFABytes8<const N: usize>(pub [u8; N]);

    #[repr(align(2))]
    pub struct DenseDFABytes16<const N: usize>(pub [u8; N]);

    #[repr(align(4))]
    pub struct DenseDFABytes32<const N: usize>(pub [u8; N]);
}

#[cfg(feature = "build")]
pub fn write_regex<W: std::io::Write>(
    name: &str,
    mut re: &str,
    mut out: W,
) -> Result<(), Box<dyn std::error::Error>> {
    use regex_automata::RegexBuilder;

    let mut anchored = false;
    if let Some(re2) = re.strip_prefix('^') {
        re = re2;
        anchored = true;
    }

    let re = RegexBuilder::new()
        .minimize(true)
        .ignore_whitespace(true)
        .unicode(true)
        .anchored(anchored)
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
        out,
        r#"
        pub static {name}: once_cell::sync::Lazy<Regex<DenseDFA<&'static [u{size}], u{size}>>> = once_cell::sync::Lazy::new(|| unsafe {{
            Regex::from_dfas(
                DenseDFA::from_bytes(&regex_util::rt::DenseDFABytes{size}({forward:?}).0),
                DenseDFA::from_bytes(&regex_util::rt::DenseDFABytes{size}({reverse:?}).0)
            )
        }});"#
    )?;

    Ok(())
}
