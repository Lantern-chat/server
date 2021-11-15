#![allow(deprecated)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use timestamp::Timestamp;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("format_iso8061", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format());
    });

    c.bench_function("format_iso8061_full", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.format_full());
    });

    //c.bench_function("format_iso8061_old", |b| {
    //    let ts = black_box(time::PrimitiveDateTime::now());
    //    b.iter(|| util::time::format_iso8061_old(ts));
    //});

    c.bench_function("format_is8061_slow", |b| {
        let ts = black_box(Utc::now().naive_utc());

        b.iter(|| format_naivedatetime(ts));
    });

    //c.bench_function("parse_iso8061_regex", |b| {
    //    let ts = black_box(util::time::format_iso8061(time::PrimitiveDateTime::now()));
    //    b.iter(|| util::time::parse_iso8061_regex(&ts));
    //});

    c.bench_function("parse_iso8061_custom", |b| {
        let ts = black_box(Timestamp::now_utc().format());

        b.iter(|| Timestamp::parse(&ts));
    });

    c.bench_function("parse_iso8061_chrono", |b| {
        let ts = black_box("2021-10-17T02:03:01+00:00");

        type T = DateTime<chrono::FixedOffset>;

        b.iter(|| T::parse_from_rfc3339(&ts).unwrap());
    });

    c.bench_function("to_unix_timestamp_ms", |b| {
        let ts = black_box(Timestamp::now_utc());

        b.iter(|| ts.to_unix_timestamp_ms());
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

pub fn format_naivedatetime(dt: NaiveDateTime) -> String {
    DateTime::<Utc>::from_utc(dt, Utc).to_rfc3339_opts(SecondsFormat::Millis, true)
}
