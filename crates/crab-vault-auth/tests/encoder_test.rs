use std::collections::HashMap;

use crab_vault_auth::{
    error::AuthError, // 用于断言错误类型
    Jwt, JwtEncoder,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
struct TestPayload {
    message: String,
}

#[test]
fn encode_success_with_kid() {
    // 准备 encoder，使用 HS256 对称 secret
    let mut map: HashMap<String, EncodingKey> = HashMap::new();
    map.insert("kid1".to_string(), EncodingKey::from_secret(b"my-secret"));

    let encoder = JwtEncoder::new(map);

    // 准备用作载荷与 claims
    let payload = TestPayload {
        message: "hello".into(),
    };
    let claims = Jwt::new("issuer-1", &["aud1"], payload);

    // header 指定 alg = HS256，并且设置 kid
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("kid1".into());

    let token_res = encoder.encode(&header, &claims, "kid1");
    assert!(token_res.is_ok(), "expected encoding to succeed: {:?}", token_res);
    let token = token_res.unwrap();
    assert!(!token.is_empty());
    // token 应该包含三个部分（header.payload.sig）
    assert_eq!(token.split('.').count(), 3);
}

#[test]
fn encode_error_when_kid_missing_in_encoder() {
    let map: HashMap<String, EncodingKey> = HashMap::new();
    let encoder = JwtEncoder::new(map);

    let payload = TestPayload {
        message: "no-kid".into(),
    };
    let claims = Jwt::new("iss", &["aud"], payload);

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("nonexistent".into());

    let token_res = encoder.encode(&header, &claims, "nonexistent");
    assert!(
        matches!(token_res, Err(AuthError::InternalError(_))),
        "expected InternalError when kid not found, got: {:?}",
        token_res
    );
}

#[test]
fn encode_error_when_header_alg_mismatch_key() {
    // 使用 HMAC secret，但 header 标记为 RS256 —— 应当报错
    let mut map: HashMap<String, EncodingKey> = HashMap::new();
    map.insert("kid-hmac".to_string(), EncodingKey::from_secret(b"hmac-secret"));

    let encoder = JwtEncoder::new(map);

    let payload = TestPayload {
        message: "alg-mismatch".into(),
    };
    let claims = Jwt::new("iss", &["aud"], payload);

    // 故意用 RS256 的 header（和 HMAC key 不匹配）
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("kid-hmac".into());

    let token_res = encoder.encode(&header, &claims, "kid-hmac");
    assert!(
        token_res.is_err(),
        "expected error when header alg doesn't match key type"
    );
    // 具体的错误类型依赖 jsonwebtoken 的实现；至少应该被映射为 AuthError::InternalError
    assert!(
        matches!(token_res, Err(AuthError::InternalError(_))),
        "expected InternalError on alg mismatch, got: {:?}",
        token_res
    );
}
