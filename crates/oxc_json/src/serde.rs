use serde_json::Value;

use crate::ast::*;

impl<'a> From<&JsonValue<'a>> for Value {
    fn from(v: &JsonValue<'a>) -> Self {
        match v {
            JsonValue::Object(o) => {
                let map = o
                    .properties
                    .iter()
                    .map(|p| (p.key.value.to_string(), Value::from(&p.value)))
                    .collect();
                Value::Object(map)
            }
            JsonValue::Array(a) => Value::Array(a.elements.iter().map(Value::from).collect()),
            JsonValue::String(s) => Value::String(s.value.to_string()),
            JsonValue::Number(n) => {
                if let Ok(i) = n.raw.parse::<i64>() {
                    Value::Number(i.into())
                } else if let Ok(f) = n.raw.parse::<f64>() {
                    serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            }
            JsonValue::Bool(b) => Value::Bool(b.value),
            JsonValue::Null(_) => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::parse;

    #[test]
    fn object_to_serde() {
        let result = parse(r#"{"name": "test", "count": 42}"#);
        let value = serde_json::Value::from(&result.value);
        assert_eq!(value, json!({"name": "test", "count": 42}));
    }

    #[test]
    fn array_to_serde() {
        let result = parse(r#"[1, "two", true, null]"#);
        let value = serde_json::Value::from(&result.value);
        assert_eq!(value, json!([1, "two", true, null]));
    }

    #[test]
    fn nested_to_serde() {
        let result = parse(r#"{"a": {"b": [1, 2]}}"#);
        let value = serde_json::Value::from(&result.value);
        assert_eq!(value, json!({"a": {"b": [1, 2]}}));
    }

    #[test]
    fn numbers_integer_vs_float() {
        let result = parse("42");
        let value = serde_json::Value::from(&result.value);
        assert_eq!(value, json!(42));
        assert!(value.is_i64());

        let result = parse("3.14");
        let value = serde_json::Value::from(&result.value);
        assert_eq!(value, json!(3.14));
        assert!(value.is_f64());
    }

    #[test]
    fn escaped_string_to_serde() {
        let result = parse(r#""hello\nworld""#);
        let value = serde_json::Value::from(&result.value);
        assert_eq!(value, json!("hello\nworld"));
    }

    #[test]
    fn jsonc_to_serde() {
        let source = r#"{
            // this is a comment
            "key": "value",
        }"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let value = serde_json::Value::from(&result.value);
        assert_eq!(value, json!({"key": "value"}));
    }

    #[test]
    fn primitives_to_serde() {
        assert_eq!(serde_json::Value::from(&parse("true").value), json!(true));
        assert_eq!(serde_json::Value::from(&parse("false").value), json!(false));
        assert_eq!(serde_json::Value::from(&parse("null").value), json!(null));
    }
}
