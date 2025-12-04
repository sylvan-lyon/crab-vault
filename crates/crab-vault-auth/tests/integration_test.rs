// tests/integration_test.rs

// 只有在开启 server-side 特性时才运行这些测试，因为 JwtDecoder 需要它
#![cfg(feature = "server-side")]

use chrono::Duration;
use crab_vault_auth::{
    error::AuthError, HttpMethod, Jwt, JwtDecoder, JwtEncoder, Permission,
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 定义一个简单的自定义 Payload 用于测试泛型支持
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct UserPayload {
    username: String,
    role: String,
}

// 辅助函数：快速生成测试用的 Key Pair
fn setup_keys() -> (String, EncodingKey, DecodingKey) {
    let secret = b"super_secret_key_for_testing";
    let kid = "key_v1".to_string();
    (
        kid,
        EncodingKey::from_secret(secret),
        DecodingKey::from_secret(secret),
    )
}

// 辅助函数：构建 Encoder
fn create_encoder(kid: &str, enc_key: EncodingKey) -> JwtEncoder {
    let mut map = HashMap::new();
    map.insert(kid.to_string(), (enc_key, Algorithm::HS256));
    JwtEncoder::new(map)
}

// 辅助函数：构建 Decoder
fn create_decoder(iss: &str, kid: &str, dec_key: DecodingKey, aud: &str) -> JwtDecoder {
    let mut map = HashMap::new();
    // 注意库中定义的 Key 是 (iss, kid)
    map.insert((iss.to_string(), kid.to_string()), dec_key);
    
    JwtDecoder::new(map, &[Algorithm::HS256], &[iss], &[aud])
}

#[test]
fn test_jwt_happy_path_with_permission() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    let decoder = create_decoder("crab-vault", &kid, dec_key, "web-client");

    // 1. 创建 Payload (使用库内置的 Permission)
    let perm = Permission::new_root();
    
    // 2. 创建 JWT 对象
    let claims = Jwt::new("crab-vault", &["web-client"], perm.clone());

    // 3. 编码
    let token = encoder.encode(&claims, &kid).expect("Encoding failed");

    // 4. 解码
    let decoded_jwt = decoder
        .decode::<Permission>(&token)
        .expect("Decoding failed");

    // 5. 验证
    assert_eq!(decoded_jwt.iss, "crab-vault");
    assert_eq!(decoded_jwt.load, perm);
    assert_eq!(decoded_jwt.aud, vec!["web-client"]);
}

#[test]
fn test_jwt_custom_payload() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    let decoder = create_decoder("auth-service", &kid, dec_key, "api");

    let payload = UserPayload {
        username: "ferris".to_string(),
        role: "admin".to_string(),
    };
    
    let claims = Jwt::new("auth-service", &["api"], payload.clone());
    let token = encoder.encode(&claims, &kid).unwrap();

    let decoded = decoder.decode::<UserPayload>(&token).unwrap();
    assert_eq!(decoded.load, payload);
}

#[test]
fn test_encode_randomly() {
    let secret = b"secret";
    let mut map = HashMap::new();
    // 放入两个 key
    map.insert("k1".to_string(), (EncodingKey::from_secret(secret), Algorithm::HS256));
    map.insert("k2".to_string(), (EncodingKey::from_secret(secret), Algorithm::HS256));
    
    let encoder = JwtEncoder::new(map);
    let payload = UserPayload { username: "t".into(), role: "u".into() };
    let claims = Jwt::new("iss", &["aud"], payload);

    // 随机编码多次，确保不会 panic 且能生成 token
    for _ in 0..5 {
        let token = encoder.encode_randomly(&claims).expect("Random encode failed");
        assert!(!token.is_empty());
    }
}

#[test]
fn test_decode_unchecked() {
    let (kid, enc_key, _) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    
    let payload = UserPayload { username: "hacker".into(), role: "none".into() };
    let claims = Jwt::new("iss", &["aud"], payload);
    let token = encoder.encode(&claims, &kid).unwrap();

    // 使用不安全解码
    let json_value = JwtDecoder::decode_unchecked(&token).expect("Unchecked decode failed");
    
    // 验证能否读取到数据
    assert_eq!(json_value["load"]["username"], "hacker");
    assert_eq!(json_value["iss"], "iss");
}

#[test]
fn test_expiration_error() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    let decoder = create_decoder("iss", &kid, dec_key, "aud").leeway(0);

    let payload = UserPayload { username: "u".into(), role: "r".into() };
    
    // 创建一个已经过期的 Token (1秒前过期)
    let claims = Jwt::new("iss", &["aud"], payload)
        .expires_in(Duration::seconds(-1));
    
    let token = encoder.encode(&claims, &kid).unwrap();

    let result = decoder.decode::<UserPayload>(&token);
    
    match result {
        Err(AuthError::TokenExpired) => assert!(true), // 预期结果
        _ => panic!("Should have returned TokenExpired error, got {:?}", result),
    }
}

#[test]
fn test_not_before_error() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    let decoder = create_decoder("iss", &kid, dec_key, "aud").leeway(0);

    let payload = UserPayload { username: "u".into(), role: "r".into() };

    // 创建一个未来才生效的 Token (1分钟后生效)
    let claims = Jwt::new("iss", &["aud"], payload)
        .not_valid_in(Duration::seconds(60));

    let token = encoder.encode(&claims, &kid).unwrap();

    let result = decoder.decode::<UserPayload>(&token);

    match result {
        Err(AuthError::TokenNotYetValid) => assert!(true), // 预期结果
        _ => panic!("Should have returned TokenNotYetValid error, got {:?}", result),
    }
}

#[test]
fn test_issuer_mismatch() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    
    // Decoder 期待 issuer 是 "valid-issuer"
    let decoder = create_decoder("valid-issuer", &kid, dec_key, "aud");

    let payload = UserPayload { username: "u".into(), role: "r".into() };
    
    // Token 实际 issuer 是 "wrong-issuer"
    let claims = Jwt::new("wrong-issuer", &["aud"], payload);
    let token = encoder.encode(&claims, &kid).unwrap();

    let result = decoder.decode::<UserPayload>(&token);

    // 这里的逻辑：
    // decode 函数首先根据 header 中的 kid 和 payload 中的 iss 去 map 里找 key。
    // 如果 iss 不匹配，map.get(&(body.iss, kid)) 就会失败，返回 InvalidIssuer。
    // 即使 map 里有，validation 步骤也会再次检查 issuer。
    match result {
        Err(AuthError::InvalidIssuer) => assert!(true),
        _ => panic!("Should fail with InvalidIssuer, got {:?}", result),
    }
}

#[test]
fn test_audience_mismatch() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    
    // Decoder 期待 audience 是 "my-api"
    let decoder = create_decoder("iss", &kid, dec_key, "my-api");

    let payload = UserPayload { username: "u".into(), role: "r".into() };
    
    // Token 实际 aud 是 "other-app"
    let claims = Jwt::new("iss", &["other-app"], payload);
    let token = encoder.encode(&claims, &kid).unwrap();

    let result = decoder.decode::<UserPayload>(&token);

    match result {
        Err(AuthError::InvalidAudience) => assert!(true),
        _ => panic!("Should fail with InvalidAudience, got {:?}", result),
    }
}

#[test]
fn test_wrong_kid_error() {
    // 场景：Token 使用了 kid="k1" 签名，但 Decoder 只有 kid="k2" 的公钥
    let enc_key = EncodingKey::from_secret(b"secret1");
    let mut enc_map = HashMap::new();
    enc_map.insert("k1".to_string(), (enc_key, Algorithm::HS256));
    let encoder = JwtEncoder::new(enc_map);

    let dec_key = DecodingKey::from_secret(b"secret2");
    let mut dec_map = HashMap::new();
    dec_map.insert(("iss".to_string(), "k2".to_string()), dec_key);
    // 注意：这里我们故意没有把 ("iss", "k1") 放入 decoder map
    
    let decoder = JwtDecoder::new(dec_map, &[Algorithm::HS256], &["iss"], &["aud"]);

    let claims = Jwt::new("iss", &["aud"], UserPayload { username: "u".into(), role: "r".into() });
    let token = encoder.encode(&claims, "k1").unwrap();

    let result = decoder.decode::<UserPayload>(&token);

    // 因为 decoder 找不到 ("iss", "k1") 对应的 key
    match result {
        Err(AuthError::InvalidIssuer) => assert!(true), // 库的逻辑是找不到 Key 时报 InvalidIssuer
        _ => panic!("Should fail with InvalidIssuer (key not found), got {:?}", result),
    }
}

#[test]
fn test_leeway_configuration() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    
    // 1. 创建一个刚刚过期的 Token (过期 5 秒)
    let claims = Jwt::new("iss", &["aud"], UserPayload { username: "u".into(), role: "r".into() })
        .expires_in(Duration::seconds(-5));
    let token = encoder.encode(&claims, &kid).unwrap();

    // 2. 默认 Decoder (leeway=60s) 应该能通过
    let decoder_default = create_decoder("iss", &kid, dec_key.clone(), "aud");
    assert!(decoder_default.decode::<UserPayload>(&token).is_ok(), "Default leeway should allow slightly expired tokens");

    // 3. 严格 Decoder (leeway=0s) 应该拒绝
    let decoder_strict = create_decoder("iss", &kid, dec_key, "aud")
        .leeway(0); // 设置 leeway 为 0
    
    match decoder_strict.decode::<UserPayload>(&token) {
        Err(AuthError::TokenExpired) => assert!(true),
        res => panic!("Strict decoder should reject, got {:?}", res),
    }
}

#[test]
fn test_reject_tokens_expiring_soon() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);

    // Token 还有 5 分钟过期
    let claims = Jwt::new("iss", &["aud"], UserPayload { username: "u".into(), role: "r".into() })
        .expires_in(Duration::minutes(5));
    let token = encoder.encode(&claims, &kid).unwrap();

    // Decoder 拒绝所有 10 分钟内过期的 Token
    let decoder = create_decoder("iss", &kid, dec_key, "aud")
        .reject_tokens_expiring_in_less_than(10 * 60);

    // jsonwebtoken 库的逻辑比较特殊，reject_tokens_expiring_in_less_than 不会返回 TokenExpired，
    // 而是属于 InvalidToken 或 Generic 验证失败，但在你的封装中，
    // 如果是验证参数设置导致的失败，通常会抛出 InvalidToken 或 InternalError。
    // 让我们观察具体的行为。
    let result = decoder.decode::<UserPayload>(&token);
    assert!(result.is_err(), "Should reject token expiring soon");
}

#[test]
fn test_permission_logic() {
    // 这主要是测试 Permission 结构体本身的方法逻辑，但也属于集成的一部分
    
    // 1. Root Permission
    let root = Permission::new_root();
    let compiled_root = root.compile();
    assert!(compiled_root.can_perform_method(HttpMethod::Get));
    assert!(compiled_root.can_perform_method(HttpMethod::Delete));
    assert!(compiled_root.can_access("/any/path"));
    assert!(compiled_root.check_size(99999999));
    assert!(compiled_root.check_content_type("application/json"));

    // 2. Minimum Permission
    let min = Permission::new_minimum();
    let compiled_min = min.compile();
    assert!(!compiled_min.can_perform_method(HttpMethod::Get));
    assert!(!compiled_min.can_access("/any/path"));
    assert!(compiled_min.check_size(0));
    assert!(!compiled_min.check_size(1));

    // 3. Custom Permission
    let custom = Permission::new()
        .permit_method(vec![HttpMethod::Get])
        .permit_resource_pattern("/api/v1/*")
        .restrict_maximum_size(1024)
        .permit_content_type(vec!["image/png".to_string()]);
    
    let compiled = custom.compile();
    
    assert!(compiled.can_perform_method(HttpMethod::Get));
    assert!(!compiled.can_perform_method(HttpMethod::Post)); // 只读
    
    assert!(compiled.can_access("/api/v1/users"));
    assert!(!compiled.can_access("/api/v2/users"));
    
    assert!(compiled.check_size(1000));
    assert!(!compiled.check_size(1025));

    assert!(compiled.check_content_type("image/png"));
    assert!(!compiled.check_content_type("image/jpeg"));
}

#[test]
fn test_multiple_audience() {
    let (kid, enc_key, dec_key) = setup_keys();
    let encoder = create_encoder(&kid, enc_key);
    
    // 允许多个 audience
    let decoder = create_decoder("iss", &kid, dec_key, "service-a")
        .possible_audience(&["service-a", "service-b"]);

    // Token 面向 service-b
    let claims = Jwt::new("iss", &["service-b"], UserPayload { username: "u".into(), role: "r".into() });
    let token = encoder.encode(&claims, &kid).unwrap();

    assert!(decoder.decode::<UserPayload>(&token).is_ok());
}