mod ast;
mod parser;

pub use ast::*;
pub use parser::{JsonParseOptions, JsonParseReturn, JsonParser};

/// Parse JSON/JSONC source text with default options (JSONC mode).
pub fn parse(source: &str) -> JsonParseReturn<'_> {
    JsonParser::new(source, JsonParseOptions::default()).parse()
}

/// Parse strict JSON source text (no comments, no trailing commas).
pub fn parse_json(source: &str) -> JsonParseReturn<'_> {
    JsonParser::new(source, JsonParseOptions::strict()).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object() {
        let result = parse("{}");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let obj = result.value.as_object().unwrap();
        assert!(obj.properties.is_empty());
        assert_eq!(obj.span.start, 0);
        assert_eq!(obj.span.end, 2);
    }

    #[test]
    fn empty_array() {
        let result = parse("[]");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let arr = result.value.as_array().unwrap();
        assert!(arr.elements.is_empty());
        assert_eq!(arr.span.start, 0);
        assert_eq!(arr.span.end, 2);
    }

    #[test]
    fn string_value() {
        let result = parse(r#""hello""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("hello"));
        assert_eq!(result.value.span().start, 0);
        assert_eq!(result.value.span().end, 7);
    }

    #[test]
    fn number_values() {
        for (input, expected) in [("42", 42.0), ("-1", -1.0), ("3.14", 3.14), ("1e10", 1e10)] {
            let result = parse(input);
            assert!(result.errors.is_empty(), "{input}: {:?}", result.errors);
            if let JsonValue::Number(n) = &result.value {
                assert_eq!(n.as_f64(), Some(expected), "{input}");
                assert_eq!(n.span.start, 0);
                assert_eq!(n.span.end, input.len() as u32);
            } else {
                panic!("{input}: expected number, got {:?}", result.value);
            }
        }
    }

    #[test]
    fn bool_values() {
        let result = parse("true");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_bool(), Some(true));

        let result = parse("false");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_bool(), Some(false));
    }

    #[test]
    fn null_value() {
        let result = parse("null");
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(matches!(result.value, JsonValue::Null(_)));
        assert_eq!(result.value.span().start, 0);
        assert_eq!(result.value.span().end, 4);
    }

    #[test]
    fn simple_object() {
        let source = r#"{"name": "test", "version": "1.0.0"}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.properties.len(), 2);

        let name = &obj.properties[0];
        assert_eq!(name.key.value, "name");
        assert_eq!(name.value.as_str(), Some("test"));
        // Key span includes quotes
        assert_eq!(&source[name.key.span.start as usize..name.key.span.end as usize], "\"name\"");

        let version = &obj.properties[1];
        assert_eq!(version.key.value, "version");
        assert_eq!(version.value.as_str(), Some("1.0.0"));
    }

    #[test]
    fn nested_object() {
        let source = r#"{"a": {"b": 1}}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let obj = result.value.as_object().unwrap();
        let inner = obj.properties[0].value.as_object().unwrap();
        assert_eq!(inner.properties.len(), 1);
        assert_eq!(inner.properties[0].key.value, "b");
    }

    #[test]
    fn array_of_values() {
        let source = r#"[1, "two", true, null]"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let arr = result.value.as_array().unwrap();
        assert_eq!(arr.elements.len(), 4);
        assert!(matches!(arr.elements[0], JsonValue::Number(_)));
        assert_eq!(arr.elements[1].as_str(), Some("two"));
        assert_eq!(arr.elements[2].as_bool(), Some(true));
        assert!(matches!(arr.elements[3], JsonValue::Null(_)));
    }

    #[test]
    fn string_with_escapes() {
        let source = r#""hello \"world\"""#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        // The value includes the raw escape sequences for now
        assert_eq!(result.value.as_str(), Some(r#"hello \"world\""#));
    }

    #[test]
    fn object_get_helper() {
        let source = r#"{"name": "test"}"#;
        let result = parse(source);
        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.get_value("name").and_then(|v| v.as_str()), Some("test"));
        assert!(obj.get_value("missing").is_none());
    }

    // --- JSONC tests ---

    #[test]
    fn line_comments() {
        let source = "{\n  // this is a comment\n  \"key\": \"value\"\n}";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.properties.len(), 1);
        assert_eq!(obj.properties[0].key.value, "key");

        assert_eq!(result.comments.len(), 1);
        assert_eq!(result.comments[0].kind, JsonCommentKind::Line);
        assert_eq!(result.comments[0].value, " this is a comment");
    }

    #[test]
    fn block_comments() {
        let source = "{ /* block */ \"key\": \"value\" }";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.properties.len(), 1);

        assert_eq!(result.comments.len(), 1);
        assert_eq!(result.comments[0].kind, JsonCommentKind::Block);
        assert_eq!(result.comments[0].value, " block ");
    }

    #[test]
    fn trailing_comma_object() {
        let source = r#"{"a": 1, "b": 2,}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.properties.len(), 2);
    }

    #[test]
    fn trailing_comma_array() {
        let source = "[1, 2, 3,]";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let arr = result.value.as_array().unwrap();
        assert_eq!(arr.elements.len(), 3);
    }

    // --- Strict mode tests ---

    #[test]
    fn strict_rejects_comments() {
        let source = "{ // comment\n\"key\": 1 }";
        let result = parse_json(source);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn strict_rejects_trailing_commas() {
        let source = r#"{"a": 1,}"#;
        let result = parse_json(source);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn strict_parses_valid_json() {
        let source = r#"{"name": "test", "count": 42}"#;
        let result = parse_json(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.properties.len(), 2);
    }

    // --- Span accuracy tests ---

    #[test]
    fn property_spans() {
        let source = r#"{ "key": "value" }"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let obj = result.value.as_object().unwrap();
        let prop = &obj.properties[0];
        // Property span: from key start to value end
        assert_eq!(&source[prop.span.start as usize..prop.span.end as usize], "\"key\": \"value\"");
        // Key span: includes quotes
        assert_eq!(&source[prop.key.span.start as usize..prop.key.span.end as usize], "\"key\"");
        // Value span: includes quotes
        assert_eq!(
            &source[prop.value.span().start as usize..prop.value.span().end as usize],
            "\"value\""
        );
    }

    #[test]
    fn whitespace_preserved_in_spans() {
        let source = "  42  ";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        // Number span should not include leading/trailing whitespace
        assert_eq!(result.value.span().start, 2);
        assert_eq!(result.value.span().end, 4);
    }

    // --- Package.json-like test ---

    #[test]
    fn package_json() {
        let source = r#"{
  "name": "my-package",
  "version": "1.0.0",
  "main": "./dist/index.js",
  "exports": {
    ".": {
      "import": "./dist/index.mjs",
      "require": "./dist/index.cjs"
    }
  },
  "dependencies": {
    "foo": "^1.0.0"
  }
}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);

        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.properties.len(), 5);
        assert_eq!(obj.get_value("name").and_then(|v| v.as_str()), Some("my-package"));
        assert_eq!(obj.get_value("version").and_then(|v| v.as_str()), Some("1.0.0"));

        let exports = obj.get_value("exports").unwrap().as_object().unwrap();
        let dot = exports.get_value(".").unwrap().as_object().unwrap();
        assert_eq!(dot.get_value("import").and_then(|v| v.as_str()), Some("./dist/index.mjs"));
    }
}
