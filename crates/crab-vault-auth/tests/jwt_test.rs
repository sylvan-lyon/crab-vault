use crab_vault_auth::error::AuthError;
use crab_vault_auth::{HttpMethod, Jwt, JwtConfig, Permission};

use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct UserProfile {
    user_name: String,
    roles: Vec<String>,
}

fn setup_config(
    alg: Algorithm,
    secret: &'static str,
    validate_issuer: bool,
    validate_audience: bool,
) -> JwtConfig {
    let mut decoding_key_map = HashMap::new();
    decoding_key_map.insert(alg, DecodingKey::from_secret(secret.as_bytes()));

    let mut validation = Validation::new(alg);
    validation.validate_exp = true;
    validation.validate_nbf = true;

    // 这里因为测试，所以需要严格控制一个 token 的生命周期
    validation.leeway = 0;

    if validate_issuer {
        validation.set_issuer(&["test-issuer"]);
    }

    if validate_audience {
        validation.set_audience(&["test-audience"]);
    } else {
        validation.validate_aud = false;
    }

    JwtConfig {
        encoding_key: EncodingKey::from_secret(secret.as_bytes()),
        decoding_key: decoding_key_map,
        header: Header::new(alg),
        validation,
        uuid_generation: Uuid::new_v4
    }
}

#[test]
fn test_encode_decode_happy_path() {
    let config = setup_config(Algorithm::HS256, "secret", false, false);
    let payload = UserProfile {
        user_name: "test user".to_string(),
        roles: vec!["user".to_string()],
    };

    let claims = Jwt::new(payload.clone()).expires_in(Duration::minutes(5));

    let token = Jwt::encode(&claims, &config).expect("Encoding failed");
    let decoded_claims = Jwt::<UserProfile>::decode(&token, &config).expect("Decoding failed");

    assert_eq!(claims.payload, payload);
    assert_eq!(decoded_claims.payload, payload);
    assert_eq!(decoded_claims.jti, claims.jti);
}

#[test]
fn test_decode_expired_token() {
    let config = setup_config(Algorithm::HS256, "secret", false, false);
    let payload = UserProfile {
        user_name: "test user".to_string(),
        roles: vec![],
    };

    // 创建一个 1 秒前就已过期的 token
    let claims = Jwt::new(payload).expires_in(Duration::seconds(-1));
    let token = Jwt::encode(&claims, &config).unwrap();

    // 等待一小会儿确保时间戳差异足够大
    std::thread::sleep(std::time::Duration::from_secs(1));

    let result = Jwt::<UserProfile>::decode(&token, &config);
    assert!(matches!(result, Err(AuthError::TokenExpired)));
}

#[test]
fn test_decode_premature_token() {
    let mut config = setup_config(Algorithm::HS256, "secret", false, false);
    // 需要验证 nbf 字段
    config.validation.validate_nbf = true;

    let payload = UserProfile {
        user_name: "test user".to_string(),
        roles: vec![],
    };

    // 创建一个 2 秒后才生效的 token
    let claims = Jwt::new(payload).not_valid_in(Duration::seconds(2));
    let token = Jwt::encode(&claims, &config).unwrap();

    println!("{token}");

    let result = Jwt::<UserProfile>::decode(&token, &config);
    assert!(matches!(result, Err(AuthError::TokenNotYetValid)));

    // 休眠一秒后查看
    std::thread::sleep(std::time::Duration::new(1, 0));
    let result = Jwt::<UserProfile>::decode(&token, &config);
    assert!(matches!(result, Err(AuthError::TokenNotYetValid)));

    // 再休眠一秒后查看，现在是两秒后
    std::thread::sleep(std::time::Duration::new(1, 0));
    let result = Jwt::<UserProfile>::decode(&token, &config);
    assert!(matches!(result, Ok(_)));
}

#[test]
fn test_decode_invalid_signature() {
    let config_good = setup_config(Algorithm::HS256, "correct_secret", false, false);
    let config_bad = setup_config(Algorithm::HS256, "wrong_secret", false, false);
    let payload = UserProfile {
        user_name: "test user".to_string(),
        roles: vec![],
    };

    let claims = Jwt::new(payload);
    let token = Jwt::encode(&claims, &config_good).unwrap();

    // 使用错误的 secret 解码
    let result = Jwt::<UserProfile>::decode(&token, &config_bad);
    assert!(matches!(result, Err(AuthError::InvalidSignature)));
}

#[test]
fn test_decode_invalid_issuer() {
    let config = setup_config(Algorithm::HS256, "secret", true, false); // 开启 issuer 验证
    let payload = UserProfile {
        user_name: "test user".to_string(),
        roles: vec![],
    };

    // 签发一个 issuer 不匹配的 token
    let claims = Jwt::new(payload).issue_as("wrong-issuer");
    let token = Jwt::encode(&claims, &config).unwrap();

    let result = Jwt::<UserProfile>::decode(&token, &config);
    assert!(matches!(result, Err(AuthError::InvalidIssuer)));
}

#[test]
fn test_decode_valid_issuer() {
    let config = setup_config(Algorithm::HS256, "secret", true, false); // 开启 issuer 验证
    let payload = UserProfile {
        user_name: "test user".to_string(),
        roles: vec![],
    };

    // 签发一个 issuer 匹配的 token
    let claims = Jwt::new(payload.clone()).issue_as("test-issuer");
    let token = Jwt::encode(&claims, &config).unwrap();

    let decoded = Jwt::<UserProfile>::decode(&token, &config).unwrap();
    assert_eq!(decoded.payload, payload);
}

#[test]
fn test_decode_invalid_audience() {
    let config = setup_config(Algorithm::HS256, "secret", false, true); // 开启 audience 验证
    let payload = UserProfile {
        user_name: "test user".to_string(),
        roles: vec![],
    };

    // 签发一个 audience 不匹配的 token
    let claims = Jwt::new(payload).audiences(&["wrong-audience".to_string()]);
    let token = Jwt::encode(&claims, &config).unwrap();

    let result = Jwt::<UserProfile>::decode(&token, &config);
    assert!(matches!(result, Err(AuthError::InvalidAudience)));
}

#[test]
fn test_decode_unchecked() {
    let config = setup_config(Algorithm::HS256, "secret", false, false);
    let payload = UserProfile {
        user_name: "unchecked_user".to_string(),
        roles: vec!["admin".to_string(), "auditor".to_string()],
    };

    let claims = Jwt::new(payload.clone());
    let token = Jwt::encode(&claims, &config).unwrap();

    println!("{token}");

    let decoded_value =
        Jwt::<UserProfile>::decode_unchecked(&token).expect("Unchecked decode failed");

    let decoded_payload = serde_json::from_value::<Jwt<UserProfile>>(decoded_value.clone())
        .unwrap()
        .payload;

    assert_eq!(decoded_payload, payload);
    assert_eq!(
        Uuid::from_str(decoded_value["jti"].as_str().unwrap()).unwrap(),
        claims.jti
    );
}

#[test]
fn test_permission_logic() {
    // 1. Root permission
    let root_perm = Permission::new_root();
    assert!(root_perm.can_perform(HttpMethod::Get));
    assert!(root_perm.can_perform(HttpMethod::Post));
    assert!(root_perm.can_access("/any/path/is/ok"));
    assert!(root_perm.check_size(u64::MAX));
    assert!(root_perm.check_content_type("application/anything"));

    // 2. Specific permissions
    let specific_perm = Permission {
        operations: vec![HttpMethod::Get, HttpMethod::Post],
        resource_pattern: "/users/*".to_string(),
        conditions: crab_vault_auth::Conditions {
            max_size: Some(1024),
            allowed_content_types: vec!["image/png".to_string(), "image/jpeg".to_string()],
        },
    };

    // can_perform
    assert!(specific_perm.can_perform(HttpMethod::Get));
    assert!(specific_perm.can_perform(HttpMethod::Post));
    assert!(!specific_perm.can_perform(HttpMethod::Delete));

    // can_access
    assert!(specific_perm.can_access("/users/123"));
    assert!(specific_perm.can_access("/users/abc-def"));
    assert!(!specific_perm.can_access("/users")); // glob * 需要至少一个字符
    assert!(!specific_perm.can_access("/posts/123"));

    // check_size
    assert!(specific_perm.check_size(512));
    assert!(specific_perm.check_size(1024));
    assert!(!specific_perm.check_size(1025));

    // check_content_type
    assert!(specific_perm.check_content_type("image/png"));
    assert!(specific_perm.check_content_type("image/jpeg"));
    assert!(!specific_perm.check_content_type("image/gif"));
    assert!(!specific_perm.check_content_type("application/json"));
}

#[test]
fn test_error_response_mapping() {
    // 401 Unauthorized errors
    let unauthorized_errors = vec![
        AuthError::MissingAuthHeader,
        AuthError::InvalidAuthFormat,
        AuthError::TokenInvalid,
        AuthError::TokenExpired,
        AuthError::TokenNotYetValid,
        AuthError::InvalidSignature,
    ];
    for error in unauthorized_errors {
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    // 403 Forbidden
    let forbidden_error = AuthError::InsufficientPermissions;
    let response = forbidden_error.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);

    // 500 Internal Server Error
    let internal_error = AuthError::InternalError("db connection failed".to_string());
    let response = internal_error.into_response();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn test_jwt_builder_methods() {
    let now = Utc::now();
    let expiry = now + Duration::hours(1);
    let not_before = now + Duration::minutes(1);

    let claims = Jwt::new("my-payload".to_string())
        .issue_as("my-issuer")
        .audiences(&["aud1".to_string(), "aud2".to_string()])
        .expires_at(expiry)
        .not_valid_till(not_before);

    assert_eq!(claims.payload, "my-payload");
    assert_eq!(claims.iss, Some("my-issuer".to_string()));
    assert_eq!(claims.aud, vec!["aud1".to_string(), "aud2".to_string()]);
    assert_eq!(claims.exp, expiry.timestamp());
    assert_eq!(claims.nbf, not_before.timestamp());
}
