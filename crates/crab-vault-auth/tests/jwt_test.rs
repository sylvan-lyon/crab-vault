#![cfg(feature = "server-side")]

use crab_vault_auth::{error::AuthError, HttpMethod, Jwt, JwtConfig, Permission};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

// 1. 定义一个自定义的载荷结构体 (Custom Payload Struct)
//    这个结构体将作为 JWT 的 `lad` 字段，需要实现序列化和反序列化。
#[derive(Serialize, Deserialize, Debug, Clone)]
struct MyPayload {
    user_id: Uuid,
    username: String,
    permissions: Permission,
}

// 2. 辅助函数：创建一套用于测试的 JwtConfig
//    返回配置以及相关的 issuer, audience, kid，方便在测试中断言。
fn setup_jwt_config() -> (JwtConfig, String, String, String) {
    let issuer = "crab-vault-test-issuer".to_string();
    let audience = "crab-vault-test-audience".to_string();
    let kid = "test_key_id_1".to_string();
    let secret = "a_very_secure_secret_key_for_testing_hs256_algorithm";

    // 创建 JWT 头部，指定算法和密钥 ID (kid)
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(kid.clone());

    // 创建用于签名的 EncodingKey
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    // 创建用于验证的 DecodingKey 映射
    // 结构为：{ issuer -> { kid -> DecodingKey } }
    let mut decoding_keys_for_issuer = HashMap::new();
    decoding_keys_for_issuer.insert(
        kid.clone(),
        DecodingKey::from_secret(secret.as_bytes()),
    );
    let mut decoding_key_map = HashMap::new();
    decoding_key_map.insert(issuer.clone(), decoding_keys_for_issuer);

    // 创建验证规则
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[issuer.clone()]);
    validation.set_audience(&[audience.clone()]);
    validation.leeway = 0;
    validation.validate_nbf = true;

    let config = JwtConfig {
        header,
        encoding_key,
        decoding_key: decoding_key_map,
        validation,
    };

    (config, issuer, audience, kid)
}

// 3. 主测试用例：测试 JWT 从生成到验证的完整成功流程
#[test]
fn test_jwt_full_cycle_success() {
    // --- 准备阶段 ---
    let (config, issuer, audience, _kid) = setup_jwt_config();
    let user_id = Uuid::new_v4();

    // 创建一个权限对象
    let permissions = Permission::new()
        .permit_method(vec![HttpMethod::Get, HttpMethod::Post])
        .permit_resource_pattern("/data/private/*")
        .restrict_maximum_size(1024 * 1024) // 1MB
        .permit_content_type(vec!["image/jpeg".to_string(), "application/json".to_string()]);

    // 创建自定义载荷
    let payload = MyPayload {
        user_id,
        username: "test_user".to_string(),
        permissions: permissions.clone(),
    };

    // 创建 JWT，并设置 5 分钟后过期
    let original_jwt = Jwt::new(issuer, vec![audience], payload.clone())
        .expires_in(chrono::Duration::try_minutes(5).unwrap());

    // --- 编码阶段 ---
    let token_string = Jwt::encode(&original_jwt, &config).expect("编码 JWT 失败");
    assert!(!token_string.is_empty(), "生成的 token 字符串不应为空");

    // --- 解码阶段 ---
    let decoded_jwt =
        Jwt::<MyPayload>::decode(&token_string, &config).expect("解码并验证 JWT 失败");

    // --- 断言验证阶段 ---
    assert_eq!(original_jwt.iss, decoded_jwt.iss, "签发者(iss)不匹配");
    assert_eq!(original_jwt.aud, decoded_jwt.aud, "受众(aud)不匹配");
    assert_eq!(original_jwt.exp, decoded_jwt.exp, "过期时间(exp)不匹配");
    assert_eq!(original_jwt.jti, decoded_jwt.jti, "JWT ID(jti)不匹配");
    assert_eq!(
        original_jwt.lad.user_id, decoded_jwt.lad.user_id,
        "载荷中的 user_id 不匹配"
    );
    assert_eq!(
        original_jwt.lad.username, decoded_jwt.lad.username,
        "载荷中的 username 不匹配"
    );

    // 详细验证权限对象
    let decoded_permissions = decoded_jwt.lad.permissions;
    assert_eq!(permissions.methods, decoded_permissions.methods);
    assert_eq!(
        permissions.resource_pattern,
        decoded_permissions.resource_pattern
    );
    assert_eq!(permissions.max_size, decoded_permissions.max_size);
    assert_eq!(
        permissions.allowed_content_types,
        decoded_permissions.allowed_content_types
    );
}

// 4. 失败场景测试用例

#[test]
fn test_jwt_decode_expired() {
    // --- 准备 ---
    let (config, issuer, audience, _kid) = setup_jwt_config();
    let payload = MyPayload {
        user_id: Uuid::new_v4(),
        username: "expired_user".to_string(),
        permissions: Permission::new(),
    };

    // 创建一个在 1 秒前就已过期的 JWT
    let expired_jwt =
        Jwt::new(issuer, vec![audience], payload).expires_in(chrono::Duration::seconds(-1));

    let token_string = Jwt::encode(&expired_jwt, &config).unwrap();

    // 等待 2 秒，确保时间检查一定会失败 (即使有 leeway)
    sleep(Duration::from_secs(2));

    // --- 解码并验证错误 ---
    let result = Jwt::<MyPayload>::decode(&token_string, &config);
    assert!(
        matches!(result, Err(AuthError::TokenExpired)),
        "期望得到 TokenExpired 错误，但得到了 {:?}",
        result
    );
}

#[test]
fn test_jwt_decode_not_yet_valid() {
    // --- 准备 ---
    let (config, issuer, audience, _kid) = setup_jwt_config();
    let payload = MyPayload {
        user_id: Uuid::new_v4(),
        username: "future_user".to_string(),
        permissions: Permission::new(),
    };

    // 创建一个 10 秒后才生效的 JWT
    let future_jwt =
        Jwt::new(issuer, vec![audience], payload).not_valid_in(chrono::Duration::seconds(10));

    let token_string = Jwt::encode(&future_jwt, &config).unwrap();

    // --- 解码并验证错误 ---
    let result = Jwt::<MyPayload>::decode(&token_string, &config);
    println!("now {}", chrono::Utc::now().timestamp());
    assert!(
        matches!(result, Err(AuthError::TokenNotYetValid)),
        "期望得到 TokenNotYetValid 错误，但得到了 {:?}",
        result
    );
}

// #[test]
// fn test_jwt_decode_invalid_signature() {
//     // --- 准备 ---
//     let (config, issuer, audience, _kid) = setup_jwt_config();
//     let payload = MyPayload {
//         user_id: Uuid::new_v4(),
//         username: "test_user".to_string(),
//         permissions: Permission::new(),
//     };
//     let jwt = Jwt::new(issuer, vec![audience], payload);
//     let token_string = Jwt::encode(&jwt, &config).unwrap();

//     // 使用一个包含错误密钥的解码配置
//     let (mut wrong_config, _, _, _) = setup_jwt_config();
//     let wrong_secret = "this_is_the_wrong_secret_key";
//     let kid = wrong_config.header.kid.as_ref().unwrap().clone();
//     let iss = wrong_config.validation.iss.as_ref().unwrap()[&0].clone();
//     let mut wrong_decoding_keys_for_issuer = HashMap::new();
//     wrong_decoding_keys_for_issuer
//         .insert(kid, DecodingKey::from_secret(wrong_secret.as_bytes()));
//     wrong_config
//         .decoding_key
//         .insert(iss, wrong_decoding_keys_for_issuer);

//     // --- 解码并验证错误 ---
//     let result = Jwt::<MyPayload>::decode(&token_string, &wrong_config);
//     assert!(
//         matches!(result, Err(AuthError::InvalidSignature)),
//         "期望得到 InvalidSignature 错误，但得到了 {:?}",
//         result
//     );
// }

#[test]
fn test_jwt_decode_invalid_issuer() {
    // --- 准备 ---
    let (mut config, issuer, audience, _kid) = setup_jwt_config();
    let payload = MyPayload {
        user_id: Uuid::new_v4(),
        username: "test_user".to_string(),
        permissions: Permission::new(),
    };
    let jwt = Jwt::new(issuer, vec![audience], payload);
    let token_string = Jwt::encode(&jwt, &config).unwrap();

    // 在验证配置中设置一个错误的 issuer
    config.validation.set_issuer(&["wrong-issuer"]);

    // --- 解码并验证错误 ---
    let result = Jwt::<MyPayload>::decode(&token_string, &config);
    assert!(
        matches!(result, Err(AuthError::InvalidIssuer)),
        "期望得到 InvalidIssuer 错误，但得到了 {:?}",
        result
    );
}

#[test]
fn test_jwt_decode_invalid_audience() {
    // --- 准备 ---
    let (mut config, issuer, audience, _kid) = setup_jwt_config();
    let payload = MyPayload {
        user_id: Uuid::new_v4(),
        username: "test_user".to_string(),
        permissions: Permission::new(),
    };
    let jwt = Jwt::new(issuer, vec![audience], payload);
    let token_string = Jwt::encode(&jwt, &config).unwrap();

    // 在验证配置中设置一个错误的 audience
    config.validation.set_audience(&["wrong-audience"]);

    // --- 解码并验证错误 ---
    let result = Jwt::<MyPayload>::decode(&token_string, &config);
    assert!(
        matches!(result, Err(AuthError::InvalidAudience)),
        "期望得到 InvalidAudience 错误，但得到了 {:?}",
        result
    );
}

#[test]
fn test_jwt_decode_missing_kid() {
    // --- 准备 ---
    let (mut config, issuer, audience, _kid) = setup_jwt_config();
    // 编码时不包含 kid
    config.header.kid = None;

    let payload = MyPayload {
        user_id: Uuid::new_v4(),
        username: "test_user".to_string(),
        permissions: Permission::new(),
    };
    let jwt = Jwt::new(issuer, vec![audience], payload);
    let token_string = Jwt::encode(&jwt, &config).unwrap();

    // --- 解码并验证错误 ---
    // 解码函数会首先尝试解析 header 并获取 kid，此时应该失败
    let result = Jwt::<MyPayload>::decode(&token_string, &config);
    let expected_error = AuthError::MissingClaim("kid".to_string());
    assert!(
        matches!(result, Err(ref e) if e.to_string() == expected_error.to_string()),
        "期望得到 MissingClaim('kid') 错误，但得到了 {:?}",
        result
    );
}

// #[test]
// fn test_jwt_decode_invalid_kid() {
//     // --- 准备 ---
//     let (config, issuer, audience, _kid) = setup_jwt_config();

//     let payload = MyPayload {
//         user_id: Uuid::new_v4(),
//         username: "test_user".to_string(),
//         permissions: Permission::new(),
//     };
//     let jwt = Jwt::new(issuer, vec![audience], payload);
//     let token_string = Jwt::encode(&jwt, &config).unwrap();

//     // 使用一个不包含正确 kid 的 config 进行解码
//     let (mut wrong_config, _, _, _) = setup_jwt_config();
//     let iss = wrong_config.validation.iss.as_ref().unwrap().get(&0).unwrap();
//     let keys_for_issuer = wrong_config.decoding_key.get_mut(iss).unwrap();
//     // 移除正确的 kid
//     keys_for_issuer.remove(wrong_config.header.kid.as_ref().unwrap());
//     // 可以选择性地插入一个错误的 kid
//     keys_for_issuer.insert(
//         "a-completely-wrong-kid".to_string(),
//         DecodingKey::from_secret("any_secret".as_bytes()),
//     );

//     // --- 解码并验证错误 ---
//     let result = Jwt::<MyPayload>::decode(&token_string, &wrong_config);
//     assert!(
//         matches!(result, Err(AuthError::InvalidKeyId)),
//         "期望得到 InvalidKeyId 错误，但得到了 {:?}",
//         result
//     );
// }

// 5. 权限编译和检查的测试
#[test]
fn test_permission_compile_and_check() {
    let permissions = Permission::new()
        .permit_method(vec![HttpMethod::Get, HttpMethod::Put])
        .permit_resource_pattern("/users/?*/profile") // 使用 glob 模式
        .restrict_maximum_size(2048)
        .permit_content_type(vec!["image/png".into(), "image/jpeg".into()]);

    // 编译权限
    let compiled_permission = permissions.compile();

    // 验证方法
    assert!(compiled_permission.can_perform_method(HttpMethod::Get));
    assert!(compiled_permission.can_perform_method(HttpMethod::Put));
    assert!(!compiled_permission.can_perform_method(HttpMethod::Post));
    assert!(!compiled_permission.can_perform_method(HttpMethod::Delete));

    // 验证路径
    assert!(compiled_permission.can_access("/users/123/profile"));
    assert!(compiled_permission.can_access("/users/abc/profile"));
    assert!(!compiled_permission.can_access("/users/123/settings"));
    assert!(!compiled_permission.can_access("/other/123/profile"));

    // 验证大小
    assert!(compiled_permission.check_size(1024));
    assert!(compiled_permission.check_size(2048));
    assert!(!compiled_permission.check_size(2049));

    // 验证内容类型
    assert!(compiled_permission.check_content_type("image/png"));
    assert!(compiled_permission.check_content_type("image/jpeg"));
    assert!(!compiled_permission.check_content_type("application/json"));

    // 测试通配符
    let wildcard_permissions = Permission::new()
        .permit_resource_pattern("/files/*")
        .permit_content_type(vec!["image/*".into()]);
    let compiled_wildcard = wildcard_permissions.compile();

    assert!(compiled_wildcard.can_access("/files/document.pdf"));
    assert!(compiled_wildcard.can_access("/files/archive.zip"));
    assert!(compiled_wildcard.can_access("/files/subdirectory/file.txt"));

    // Glob `*` 必须要有一个字符
    assert!(!compiled_wildcard.can_access("/file/"));

    assert!(compiled_wildcard.check_content_type("image/png"));
    assert!(compiled_wildcard.check_content_type("image/gif"));
    assert!(!compiled_wildcard.check_content_type("text/plain"));
}
