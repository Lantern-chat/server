#![allow(deprecated)]

use criterion::{criterion_group, criterion_main, Criterion};

static INPUT: &str = r#"
Both 👬 dense 😍 and sparse DFAs can 🦎 be 😉 serialized to raw ☠️😅 bytes, and then 🤔 cheaply deserialized.
Deserialization always 💁‍♀️ takes 💅 constant time 😋 since 👨 searching ⚓ can 🤦‍♂️ be 💰 performed 🎭💃 directly on 🔛 the raw 😷😩💩💩 serialized bytes of a DFA.
This crate was specifically 🔵🔵 designed 😋 so that 🧚‍♀️ the searching ⚓ phase of a DFA has 💤 minimal runtime requirements, and can 👄 therefore 😤😡 be 🧎 used 🎶 in 🔗 no_std environments.
While 😂 no_std environments cannot 🚷✋🚫 compile regexes, they 🏾 can 👍 deserialize pre-compiled regexes.
Since 👨 this crate builds DFAs ahead 🏻🏻 of time, 🧚⏱️ it will 💰🏼 generally out-perform the regex crate on 🦎 equivalent tasks. 🙌🤗 The performance difference 🔄 is likely 😠 not ✖️ large. 🔶⬜
However, 🤔 because 🤔🤔 of a complex set 📐 of optimizations in 👏👏 the regex crate (like ⚡ literal 🏿🏻 optimizations), an accurate performance comparison 📊 may 🐵 be 🥖 difficult 👞 to do. 🤔
Sparse DFAs provide 👋 a way ↕️ to build 📷 a DFA ahead 🏻🏻 of time 😂 that 👇 sacrifices search 🔍 performance a small 👌 amount, 🈷️🈷️🈷️ but 👹
uses much 😩😂🙀 less 🙅🏻‍♂️🙅🏻‍♂️ storage space. ☀ Potentially even 😂 less 😔 than 😽 what 😟 the regex crate uses.
This crate exposes DFAs directly, such 📶 as DenseDFA and SparseDFA, which 🙌😩 enables one 1️⃣ to do 👊🏻😡👊🏻 less 😔 work 🏢🏗️💼 in ⬇️ some 😋 cases. 💼
For 🆙💕 example, 🔥🔥 if you 👉 only 🕦🤠 need 😾 the end 🔚 of a match 🔥 and not 🚯 the start 🆕 of a match,
🤷👉 then 😵 you 👨🏻👈 can 🦎 use 😡 a DFA directly without 🚫 building 👷🚧 a Regex, which 😡👏 always 👌👉 requires 📣 a second 🥈 DFA to find 🔍🤔 the start 🏁 of a match. 🤷👉
Aside 😤 from 👉😮 choosing between 👄 dense 😍 and sparse DFAs, there 😏 are several 💯 options for 4️⃣ configuring the space 👩‍🚀 usage vs 😯 search 🔍 time 😌🕒 trade off. 📴
These 😍😱 include 📲 things 😃🥳🤡 like 🌂 choosing a smaller 👱 state 👌 identifier representation, to premultiplying state 🇺🇸 identifiers and splitting a DFA’s alphabet 🔤 into 👉👌 equivalence classes. 🤡
Finally, 🅱️ DFA minimization is also 👨 provided, 🤔💭 but 🤪 can 🚡 increase ➕🔛 compilation times 🕐😆 dramatically. 🎭😱💢
"#;

use once_cell::sync::Lazy;

pub static EMOJI_RE_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::RegexBuilder::new(
        r"
    \p{RI} \p{RI}
    | \p{Emoji}
        ( \p{EMod}
        | \x{FE0F} \x{20E3}?
        | [\x{E0020}-\x{E007E}]+ \x{E007F}
        )?
        (\x{200D}
            ( \p{RI} \p{RI}
                | \p{Emoji}
                ( \p{EMod}
                | \x{FE0F} \x{20E3}?
                | [\x{E0020}-\x{E007E}]+ \x{E007F}
                )?
            )
        )*
    ",
    )
    .ignore_whitespace(true)
    .unicode(true)
    .build()
    .unwrap()
});

/// Finds emojis in the string *and* filters out single strings of `/#*[0-9]/`
pub fn find_emojis_regex(e: &str) -> impl Iterator<Item = regex::Match> {
    EMOJI_RE_REGEX
        .find_iter(e)
        .filter(|m| !((m.end() - m.start()) == 1 && matches!(m.as_str().as_bytes()[0], b'#' | b'*' | b'0'..=b'9')))
}

fn criterion_benchmark(c: &mut Criterion) {
    assert_eq!(emoji::find_emojis(INPUT).count(), find_emojis_regex(INPUT).count());

    let mut g = c.benchmark_group("find_emojis");
    g.bench_with_input("automata", INPUT, |b, x| b.iter(|| emoji::find_emojis(x).count()));
    g.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
