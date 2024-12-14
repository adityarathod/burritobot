pub fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap()
}
