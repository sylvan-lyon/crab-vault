pub mod api;
pub mod auth;
mod extractor;
mod middleware;
pub mod server;

const USER_META_PREFIX: &str = "x-crab-vault-meta-";
