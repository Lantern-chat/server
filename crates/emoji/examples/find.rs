fn main() {
    if let Some(emoji) =
        emoji::find("\u{1F469}\u{1F3FC}\u{200D}\u{2764}\u{200D}\u{1F48B}\u{200D}\u{1F468}\u{1F3FB}")
    {
        print_chars(emoji.chars());
    }

    if let Some(emoji) = emoji::find("\u{2764}") {
        assert!(emoji::find(emoji).is_some());

        print_chars(emoji.chars());
    }
}

fn print_chars(c: std::str::Chars) {
    for c in c {
        print!("{:X} ", c as u32);
    }
    println!("");
}
