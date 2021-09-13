use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    pub static ref MC_CODE_REGEX: Regex = Regex::new("(?i)\u{00A7}[0-9A-FK-OR]").unwrap();
}
