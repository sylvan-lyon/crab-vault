use crab_vault::engine::error::{EngineError, EngineResult};

pub fn merge_json_object(
    new: serde_json::Value,
    old: serde_json::Value,
) -> EngineResult<serde_json::Value> {
    use serde_json::Value;

    let ensure_is_object_and_take_the_map = |value: Value| match value {
        Value::Object(map) => Ok(map),
        _ => Err(EngineError::InvalidArgument(
            "Should be an object".to_string(),
        )),
    };

    // 首先确保新的值必须是一个 Object ，否则返回一个 invalid argument 错误
    let new_map = ensure_is_object_and_take_the_map(new)?;

    // 如果旧的值不合法，那么直接返回合法的新值，上面已经验证
    // 否则将旧值作为基底
    let mut old = match ensure_is_object_and_take_the_map(old) {
        Err(_) => return Ok(Value::Object(new_map)),
        Ok(old) => old,
    };

    for (k, v) in new_map {
        match v {
            Value::Null => old.remove(&k),
            _ => old.insert(k, v),
        };
    }

    Ok(Value::Object(old))
}
