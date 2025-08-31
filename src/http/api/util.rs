use crab_vault_engine::error::{EngineError, EngineResult};

pub fn merge_json_value(
    new: serde_json::Value,
    old: serde_json::Value,
) -> EngineResult<serde_json::Value> {
    use serde_json::Value;

    let validate_json_value = |value: Value| match value {
        Value::Object(map) => Ok(map),
        _ => Err(EngineError::InvalidArgument(
            "Should be an object".to_string(),
        )),
    };

    // 首先确保新的值必须合法，否则返回一个 invalid argument 错误
    let new = validate_json_value(new)?;

    // 如果旧的值不合法，那么直接返回合法的新值
    // 否则将旧值作为基底
    let mut res = match validate_json_value(old) {
        Err(_) => return Ok(Value::Object(new)),
        Ok(old) => old,
    };

    for (k, v) in new {
        match v {
            Value::Null => res.remove(&k),
            _ => res.insert(k, v),
        };
    }

    Ok(Value::Object(res))
}
