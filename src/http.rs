use axum::http::HeaderName;

pub mod api;
mod extractor;
mod middleware;
pub mod server;

const X_CRAB_VAULT_USER_META: HeaderName = HeaderName::from_static("x-crab-vault-user-meta");
const X_CRAB_VAULT_CREATED_AT: HeaderName = HeaderName::from_static("x-crab-vault-created-at");
const X_CRAB_VAULT_BUCKET_NAME: HeaderName = HeaderName::from_static("x-crab-vault-bucket-name");
const X_CRAB_VAULT_OBJECT_NAME: HeaderName = HeaderName::from_static("x-crab-vault-object-name");