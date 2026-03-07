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
        assert_eq!(result.value.as_str(), Some("hello \"world\""));
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

    // --- Edge cases from json-strip-comments test suite ---

    #[test]
    fn unicode_in_strings() {
        let source = r#"{"emoji": "hello 🌍", "cjk": "你好"}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.get_value("emoji").and_then(|v| v.as_str()), Some("hello 🌍"));
        assert_eq!(obj.get_value("cjk").and_then(|v| v.as_str()), Some("你好"));
    }

    #[test]
    fn unicode_in_comments() {
        let source = "{\n  // 这是注释 🎉\n  \"key\": 1\n}";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.comments.len(), 1);
        assert_eq!(result.comments[0].value, " 这是注释 🎉");
    }

    #[test]
    fn escaped_quotes_in_strings() {
        // Simple escaped quote
        let result = parse(r#""he said \"hi\"""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("he said \"hi\""));

        // Escaped backslash before quote: \\" means literal backslash + end of string
        let result = parse(r#"{"a": "b\\"}"#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            result.value.as_object().unwrap().get_value("a").and_then(|v| v.as_str()),
            Some("b\\")
        );
    }

    #[test]
    fn comment_like_syntax_in_strings() {
        // // inside a string should not be treated as a comment
        let source = r#"{"url": "https://example.com"}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            result.value.as_object().unwrap().get_value("url").and_then(|v| v.as_str()),
            Some("https://example.com")
        );

        // /* */ inside a string should not be treated as a comment
        let source = r#"{"re": "a/* b */c"}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            result.value.as_object().unwrap().get_value("re").and_then(|v| v.as_str()),
            Some("a/* b */c")
        );
    }

    #[test]
    fn unterminated_block_comment() {
        let source = "{ /* never closed\n\"key\": 1 }";
        let result = parse(source);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn empty_input() {
        let result = parse("");
        assert!(!result.errors.is_empty()); // should error on unexpected end
    }

    #[test]
    fn whitespace_only_input() {
        let result = parse("   \n\t  ");
        assert!(!result.errors.is_empty()); // should error on unexpected end
    }

    #[test]
    fn comment_only_input() {
        let result = parse("// just a comment");
        assert!(!result.errors.is_empty()); // no value found
    }

    #[test]
    fn comments_before_colon() {
        let source = "{ \"key\" /* comment */ : \"value\" }";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.get_value("key").and_then(|v| v.as_str()), Some("value"));
        assert_eq!(result.comments.len(), 1);
    }

    #[test]
    fn comments_after_value() {
        let source = "{ \"key\": \"value\" /* after */ }";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.comments.len(), 1);
    }

    #[test]
    fn comment_at_eof() {
        let source = "{\"a\": 1}\n// trailing";
        let result = parse(source);
        // Should parse the object successfully, comment is after the value
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.comments.len(), 1);
    }

    #[test]
    fn consecutive_comments() {
        let source = "{\n  // first\n  // second\n  /* third */\n  \"key\": 1\n}";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.comments.len(), 3);
    }

    #[test]
    fn nested_comment_like_syntax() {
        // /* inside a // comment
        let source = "{\n  // has /* nested\n  \"a\": 1\n}";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.comments.len(), 1);
        assert_eq!(result.comments[0].value, " has /* nested");

        // // inside a /* */ comment
        let source = "{ /* has // nested */ \"a\": 1 }";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.comments.len(), 1);
        assert_eq!(result.comments[0].value, " has // nested ");
    }

    #[test]
    fn crlf_line_endings() {
        let source = "{\r\n  \"key\": \"value\"\r\n}";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let obj = result.value.as_object().unwrap();
        assert_eq!(obj.get_value("key").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn crlf_with_line_comment() {
        let source = "{\r\n  // comment\r\n  \"key\": 1\r\n}";
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.comments.len(), 1);
    }

    #[test]
    fn deeply_nested() {
        let source = r#"{"a": {"b": {"c": {"d": [1, [2, [3]]]}}}}"#;
        let result = parse(source);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn empty_string_value() {
        let result = parse(r#""""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some(""));
        assert_eq!(result.value.span().start, 0);
        assert_eq!(result.value.span().end, 2);
    }

    #[test]
    fn negative_and_fractional_numbers() {
        let cases = [
            ("-0", -0.0_f64),
            ("0.5", 0.5),
            ("-0.5", -0.5),
            ("1e2", 100.0),
            ("1E2", 100.0),
            ("1e+2", 100.0),
            ("1e-2", 0.01),
            ("-1.5e10", -1.5e10),
        ];
        for (input, expected) in cases {
            let result = parse(input);
            assert!(result.errors.is_empty(), "{input}: {:?}", result.errors);
            if let JsonValue::Number(n) = &result.value {
                let parsed = n.as_f64().unwrap();
                assert!(
                    (parsed - expected).abs() < f64::EPSILON || (parsed.is_nan() && expected.is_nan()),
                    "{input}: expected {expected}, got {parsed}"
                );
            } else {
                panic!("{input}: expected number");
            }
        }
    }

    // --- String escape tests ---

    #[test]
    fn escape_sequences() {
        let cases = [
            (r#""\n""#, "\n"),
            (r#""\r""#, "\r"),
            (r#""\t""#, "\t"),
            (r#""\\""#, "\\"),
            (r#""\/""#, "/"),
            (r#""\b""#, "\u{0008}"),
            (r#""\f""#, "\u{000C}"),
        ];
        for (input, expected) in cases {
            let result = parse(input);
            assert!(result.errors.is_empty(), "{input}: {:?}", result.errors);
            assert_eq!(result.value.as_str(), Some(expected), "input: {input}");
        }
    }

    #[test]
    fn unicode_escape_bmp() {
        // Basic Multilingual Plane
        let result = parse(r#""\u0041""#); // 'A'
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("A"));

        let result = parse(r#""\u00e9""#); // 'é'
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("é"));

        let result = parse(r#""\u0000""#); // null char
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("\0"));
    }

    #[test]
    fn unicode_surrogate_pairs() {
        // 𝄞 (U+1D11E MUSICAL SYMBOL G CLEF)
        let result = parse(r#""\uD834\uDD1E""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("𝄞"));

        // 😀 (U+1F600)
        let result = parse(r#""\uD83D\uDE00""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("😀"));
    }

    #[test]
    fn unpaired_high_surrogate() {
        let result = parse(r#""\uD800""#);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn unpaired_low_surrogate() {
        let result = parse(r#""\uDC00""#);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn invalid_hex_escape() {
        let result = parse(r#""\u00GG""#);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn incomplete_unicode_escape() {
        let result = parse(r#""\u00""#);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn invalid_escape_character() {
        let result = parse(r#""\q""#);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn mixed_escapes_and_text() {
        let result = parse(r#""hello\nworld\t!""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.value.as_str(), Some("hello\nworld\t!"));
    }

    #[test]
    fn no_escape_borrows_from_source() {
        let result = parse(r#""hello world""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        if let JsonValue::String(s) = &result.value {
            assert!(matches!(s.value, std::borrow::Cow::Borrowed(_)));
            assert_eq!(s.raw, "hello world");
            assert_eq!(s.value.as_ref(), "hello world");
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn escape_produces_owned() {
        let result = parse(r#""hello\nworld""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        if let JsonValue::String(s) = &result.value {
            assert!(matches!(s.value, std::borrow::Cow::Owned(_)));
            // raw preserves the escape as written
            assert_eq!(s.raw, r#"hello\nworld"#);
            // value is decoded
            assert_eq!(s.value.as_ref(), "hello\nworld");
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn raw_preserves_unicode_escapes() {
        let result = parse(r#""\u0041\u0042""#);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        if let JsonValue::String(s) = &result.value {
            assert_eq!(s.raw, r#"\u0041\u0042"#);
            assert_eq!(s.value.as_ref(), "AB");
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn lone_backslash_at_end() {
        let result = parse("\"hello\\");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn trailing_commas_strict() {
        // Trailing comma in array
        let result = parse_json("[1, 2, 3,]");
        assert!(!result.errors.is_empty());

        // Trailing comma in object
        let result = parse_json(r#"{"a": 1, "b": 2,}"#);
        assert!(!result.errors.is_empty());
    }
}
