pub fn url_root(url: &str) -> (bool, &str, &str) {
    // https: / / whatever.com /
    let root_idx = url.split('/').map(|s| s.len()).take(3).sum::<usize>();
    let root = &url[..(root_idx + 2)];
    let https = root.starts_with("https://");
    (https, root, if https { &root[8..] } else { &root[7..] })
}
