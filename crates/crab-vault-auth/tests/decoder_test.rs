#![cfg(feature = "server-side")]
use std::collections::HashMap;

use crab_vault_auth::{Jwt, JwtDecoder, JwtEncoder, error::AuthError};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
struct TestPayload {
    message: String,
}

fn build_hs256_encoder_and_decoder(secret: &[u8]) -> (JwtEncoder, JwtDecoder) {
    // 构造 EncodingKey map (用于 encoder)
    let mut enc_map: HashMap<String, EncodingKey> = HashMap::new();
    enc_map.insert("kid1".to_string(), EncodingKey::from_secret(secret));
    let encoder = JwtEncoder::new(enc_map);

    // 构造 DecodingKey mapping (iss, kid) -> DecodingKey
    let mut dec_map: HashMap<(String, String), DecodingKey> = HashMap::new();
    dec_map.insert(
        ("issuer-xyz".to_string(), "kid1".to_string()),
        DecodingKey::from_secret(secret),
    );

    // 构造一个 JwtDecoder，并设置验证参数
    let decoder =
        JwtDecoder::new(dec_map, &[Algorithm::HS256], &["issuer-xyz"], &["aud-1"]).leeway(0);

    (encoder, decoder)
}

#[test]
fn decode_success_roundtrip() {
    let secret = b"super-secret";
    let (encoder, decoder) = build_hs256_encoder_and_decoder(secret);

    let payload = TestPayload {
        message: "roundtrip".into(),
    };
    // 构建 claims：iss = "issuer-xyz", aud = ["aud-1"]
    let claims = Jwt::new("issuer-xyz", &["aud-1"], payload.clone());

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("kid1".into());

    let token = encoder
        .encode(&header, &claims, "kid1")
        .expect("encode should succeed");

    let decoded: Jwt<TestPayload> = decoder.decode(&token).expect("decode should succeed");
    assert_eq!(decoded.iss, "issuer-xyz");
    assert_eq!(decoded.aud, vec!["aud-1".to_string()]);
    assert_eq!(decoded.load, payload);
}

#[test]
fn decode_invalid_signature() {
    // 使用不同的 secret 去验证（签名无效）
    let (encoder, _) = build_hs256_encoder_and_decoder(b"signing-secret");
    // decoder 使用不同 secret
    let mut dec_map: HashMap<(String, String), DecodingKey> = HashMap::new();
    dec_map.insert(
        ("issuer-xyz".to_string(), "kid1".to_string()),
        DecodingKey::from_secret(b"different-secret"),
    );
    let decoder = JwtDecoder::new(dec_map, &[Algorithm::HS256], &["issuer-xyz"], &["aud-1"]);

    let payload = TestPayload {
        message: "bad-sig".into(),
    };
    let claims = Jwt::new("issuer-xyz", &["aud-1"], payload);

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("kid1".into());

    let token = encoder.encode(&header, &claims, "kid1").expect("encode ok");

    let err = decoder
        .decode::<TestPayload>(&token)
        .expect_err("expected invalid signature");
    assert!(matches!(err, AuthError::InvalidSignature));
}

#[test]
fn decode_expired_token() {
    let secret = b"exp-secret";
    let (encoder, decoder) = build_hs256_encoder_and_decoder(secret);

    // 构造一个已经过期的 token（exp 设为过去）
    let mut claims = Jwt::new(
        "issuer-xyz",
        &["aud-1"],
        TestPayload {
            message: "expired".into(),
        },
    );
    // 让它在 10 秒前过期
    claims.exp = chrono::Utc::now().timestamp() - 10;

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("kid1".into());

    let token = encoder.encode(&header, &claims, "kid1").expect("encode ok");

    let err = decoder
        .decode::<TestPayload>(&token)
        .expect_err("expected expired error");
    assert!(matches!(err, AuthError::TokenExpired));
}

#[test]
fn decode_nbf_not_yet_valid() {
    let secret = b"nbf-secret";
    let (encoder, decoder) = build_hs256_encoder_and_decoder(secret);

    // 构造一个 nbf 在未来的 token
    let mut claims = Jwt::new(
        "issuer-xyz",
        &["aud-1"],
        TestPayload {
            message: "future".into(),
        },
    );
    claims.nbf = (chrono::Utc::now() + chrono::Duration::seconds(60)).timestamp();

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("kid1".into());

    let token = encoder.encode(&header, &claims, "kid1").expect("encode ok");

    let err = decoder
        .decode::<TestPayload>(&token)
        .expect_err("expected not-yet-valid error");
    assert!(matches!(err, AuthError::TokenNotYetValid));
}

#[test]
fn decode_missing_kid_in_header() {
    let secret = b"mkid-secret";
    let (encoder, decoder) = build_hs256_encoder_and_decoder(secret);

    let claims = Jwt::new(
        "issuer-xyz",
        &["aud-1"],
        TestPayload {
            message: "no-kid".into(),
        },
    );

    // header 不设置 kid
    let header = Header::new(Algorithm::HS256);

    let token = encoder
        .encode(&header, &claims, "kid1")
        .expect("encode ok (kid param ignored since header has none)");

    // decoder 首先尝试从 header 读取 kid，会发现没有并返回 MissingClaim("kid")
    let err = decoder
        .decode::<TestPayload>(&token)
        .expect_err("expected missing kid error");
    assert!(matches!(err, AuthError::MissingClaim(s) if s == "kid"));
}

#[test]
fn decode_unchecked_returns_payload_without_verification() {
    let secret = b"uc-secret";
    let (encoder, _decoder) = build_hs256_encoder_and_decoder(secret);

    let payload = TestPayload {
        message: "unchecked".into(),
    };
    let claims = Jwt::new("issuer-xyz", &["aud-1"], payload.clone());

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("kid1".into());

    let token = encoder.encode(&header, &claims, "kid1").expect("encode ok");

    // decode_unchecked 是关联函数
    let value = crab_vault_auth::JwtDecoder::decode_unchecked(&token).expect("unchecked decode ok");
    // 应当含有 iss 和 load.message
    assert_eq!(value["iss"], "issuer-xyz");
    assert_eq!(value["load"]["message"], payload.message);
}
