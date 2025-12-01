pub mod api;
mod extractor;
mod middleware;
pub mod server;

const USER_META_HEADER_KEY: &str = "x-crab-vault-meta-";