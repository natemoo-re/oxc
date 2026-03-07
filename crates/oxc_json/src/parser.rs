use oxc_diagnostics::OxcDiagnostic;
use oxc_span::Span;

use crate::ast::*;

/// Options for the JSON parser.
#[derive(Debug, Clone, Copy)]
pub struct JsonParseOptions {
    /// Allow `//` and `/* */` comments.
    pub allow_comments: bool,
    /// Allow trailing commas in objects and arrays.
    pub allow_trailing_commas: bool,
}

impl Default for JsonParseOptions {
    /// Default: JSONC mode (permissive).
    fn default() -> Self {
        Self { allow_comments: true, allow_trailing_commas: true }
    }
}

impl JsonParseOptions {
    /// Strict JSON mode — no comments, no trailing commas.
    pub fn strict() -> Self {
        Self { allow_comments: false, allow_trailing_commas: false }
    }
}

/// Result of parsing JSON source text.
pub struct JsonParseReturn<'a> {
    pub value: JsonValue<'a>,
    pub comments: Vec<JsonComment<'a>>,
    pub errors: Vec<OxcDiagnostic>,
}

pub struct JsonParser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    options: JsonParseOptions,
    comments: Vec<JsonComment<'a>>,
    errors: Vec<OxcDiagnostic>,
}

impl<'a> JsonParser<'a> {
    pub fn new(source: &'a str, options: JsonParseOptions) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            options,
            comments: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn parse(mut self) -> JsonParseReturn<'a> {
        self.skip_whitespace_and_comments();
        let value = self.parse_value();
        self.skip_whitespace_and_comments();
        if self.pos < self.source.len() {
            self.error("Unexpected content after JSON value", self.pos, self.pos + 1);
        }
        JsonParseReturn { value, comments: self.comments, errors: self.errors }
    }

    fn parse_value(&mut self) -> JsonValue<'a> {
        self.skip_whitespace_and_comments();

        match self.peek() {
            Some(b'"') => JsonValue::String(self.parse_string()),
            Some(b'{') => JsonValue::Object(self.parse_object()),
            Some(b'[') => JsonValue::Array(self.parse_array()),
            Some(b't') => self.parse_true(),
            Some(b'f') => self.parse_false(),
            Some(b'n') => self.parse_null(),
            Some(b'-' | b'0'..=b'9') => JsonValue::Number(self.parse_number()),
            Some(c) => {
                let start = self.pos;
                self.advance();
                self.error(&format!("Unexpected character '{}'", char::from(c)), start, self.pos);
                JsonValue::Null(Span::new(start as u32, self.pos as u32))
            }
            None => {
                self.error("Unexpected end of input", self.pos, self.pos);
                JsonValue::Null(Span::new(self.pos as u32, self.pos as u32))
            }
        }
    }

    fn parse_object(&mut self) -> JsonObject<'a> {
        let start = self.pos;
        self.expect(b'{');
        self.skip_whitespace_and_comments();

        let mut properties = Vec::new();

        if self.peek() != Some(b'}') {
            loop {
                self.skip_whitespace_and_comments();

                if self.peek() == Some(b'}') {
                    // Trailing comma case
                    break;
                }

                let prop_start = self.pos;

                if self.peek() != Some(b'"') {
                    self.error("Expected property name", self.pos, self.pos + 1);
                    break;
                }
                let key = self.parse_string();

                self.skip_whitespace_and_comments();
                if !self.eat(b':') {
                    self.error("Expected ':'", self.pos, self.pos + 1);
                    break;
                }

                self.skip_whitespace_and_comments();
                let value = self.parse_value();
                let prop_end = self.pos;

                properties.push(JsonProperty {
                    span: Span::new(prop_start as u32, prop_end as u32),
                    key,
                    value,
                });

                self.skip_whitespace_and_comments();

                if self.peek() == Some(b',') {
                    self.advance();
                    if !self.options.allow_trailing_commas && {
                        self.skip_whitespace_and_comments();
                        self.peek() == Some(b'}')
                    } {
                        self.error(
                            "Trailing commas are not allowed in JSON",
                            self.pos - 1,
                            self.pos,
                        );
                    }
                } else {
                    break;
                }
            }
        }

        self.skip_whitespace_and_comments();
        if !self.eat(b'}') {
            self.error("Expected '}'", self.pos, self.pos + 1);
        }

        JsonObject { span: Span::new(start as u32, self.pos as u32), properties }
    }

    fn parse_array(&mut self) -> JsonArray<'a> {
        let start = self.pos;
        self.expect(b'[');
        self.skip_whitespace_and_comments();

        let mut elements = Vec::new();

        if self.peek() != Some(b']') {
            loop {
                self.skip_whitespace_and_comments();

                if self.peek() == Some(b']') {
                    // Trailing comma case
                    break;
                }

                elements.push(self.parse_value());

                self.skip_whitespace_and_comments();

                if self.peek() == Some(b',') {
                    self.advance();
                    if !self.options.allow_trailing_commas && {
                        self.skip_whitespace_and_comments();
                        self.peek() == Some(b']')
                    } {
                        self.error(
                            "Trailing commas are not allowed in JSON",
                            self.pos - 1,
                            self.pos,
                        );
                    }
                } else {
                    break;
                }
            }
        }

        self.skip_whitespace_and_comments();
        if !self.eat(b']') {
            self.error("Expected ']'", self.pos, self.pos + 1);
        }

        JsonArray { span: Span::new(start as u32, self.pos as u32), elements }
    }

    fn parse_string(&mut self) -> JsonString<'a> {
        let start = self.pos;
        self.expect(b'"');

        let content_start = self.pos;
        let mut has_escapes = false;

        loop {
            match self.peek() {
                Some(b'"') => {
                    let content_end = self.pos;
                    self.advance(); // consume closing quote
                    let span = Span::new(start as u32, self.pos as u32);

                    let value = if has_escapes {
                        // For now, return the raw escaped content.
                        // A full implementation would unescape here.
                        &self.source[content_start..content_end]
                    } else {
                        &self.source[content_start..content_end]
                    };

                    return JsonString { span, value };
                }
                Some(b'\\') => {
                    has_escapes = true;
                    self.advance(); // consume backslash
                    self.advance(); // consume escaped char
                }
                Some(_) => {
                    self.advance();
                }
                None => {
                    self.error("Unterminated string", start, self.pos);
                    return JsonString {
                        span: Span::new(start as u32, self.pos as u32),
                        value: &self.source[content_start..self.pos],
                    };
                }
            }
        }
    }

    fn parse_number(&mut self) -> JsonNumber<'a> {
        let start = self.pos;

        // Optional negative sign
        if self.peek() == Some(b'-') {
            self.advance();
        }

        // Integer part
        if self.peek() == Some(b'0') {
            self.advance();
        } else {
            self.consume_digits();
        }

        // Fractional part
        if self.peek() == Some(b'.') {
            self.advance();
            self.consume_digits();
        }

        // Exponent part
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance();
            }
            self.consume_digits();
        }

        let raw = &self.source[start..self.pos];
        JsonNumber { span: Span::new(start as u32, self.pos as u32), raw }
    }

    fn parse_true(&mut self) -> JsonValue<'a> {
        let start = self.pos;
        if self.eat_str("true") {
            JsonValue::Bool(JsonBool {
                span: Span::new(start as u32, self.pos as u32),
                value: true,
            })
        } else {
            self.error("Expected 'true'", start, self.pos);
            JsonValue::Null(Span::new(start as u32, self.pos as u32))
        }
    }

    fn parse_false(&mut self) -> JsonValue<'a> {
        let start = self.pos;
        if self.eat_str("false") {
            JsonValue::Bool(JsonBool {
                span: Span::new(start as u32, self.pos as u32),
                value: false,
            })
        } else {
            self.error("Expected 'false'", start, self.pos);
            JsonValue::Null(Span::new(start as u32, self.pos as u32))
        }
    }

    fn parse_null(&mut self) -> JsonValue<'a> {
        let start = self.pos;
        if self.eat_str("null") {
            JsonValue::Null(Span::new(start as u32, self.pos as u32))
        } else {
            self.error("Expected 'null'", start, self.pos);
            JsonValue::Null(Span::new(start as u32, self.pos as u32))
        }
    }

    // --- Helpers ---

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            if self.options.allow_comments {
                if self.starts_with(b"//") {
                    self.parse_line_comment();
                    continue;
                }
                if self.starts_with(b"/*") {
                    self.parse_block_comment();
                    continue;
                }
            } else if self.starts_with(b"//") || self.starts_with(b"/*") {
                let start = self.pos;
                self.error("Comments are not allowed in JSON", start, start + 2);
                // Still skip the comment to continue parsing
                if self.starts_with(b"//") {
                    self.parse_line_comment();
                } else {
                    self.parse_block_comment();
                }
                continue;
            }
            break;
        }
    }

    fn parse_line_comment(&mut self) {
        let start = self.pos;
        self.pos += 2; // skip //
        let content_start = self.pos;
        while self.pos < self.source.len() && self.bytes[self.pos] != b'\n' {
            self.pos += 1;
        }
        let content_end = self.pos;
        self.comments.push(JsonComment {
            span: Span::new(start as u32, self.pos as u32),
            value: &self.source[content_start..content_end],
            kind: JsonCommentKind::Line,
        });
    }

    fn parse_block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2; // skip /*
        let content_start = self.pos;
        loop {
            if self.pos + 1 >= self.source.len() {
                self.error("Unterminated block comment", start, self.pos);
                break;
            }
            if self.bytes[self.pos] == b'*' && self.bytes[self.pos + 1] == b'/' {
                let content_end = self.pos;
                self.pos += 2; // skip */
                self.comments.push(JsonComment {
                    span: Span::new(start as u32, self.pos as u32),
                    value: &self.source[content_start..content_end],
                    kind: JsonCommentKind::Block,
                });
                return;
            }
            self.pos += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.source.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r')
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn advance(&mut self) {
        if self.pos < self.source.len() {
            self.pos += 1;
        }
    }

    fn eat(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: u8) {
        if !self.eat(expected) {
            self.error(&format!("Expected '{}'", char::from(expected)), self.pos, self.pos + 1);
        }
    }

    fn eat_str(&mut self, s: &str) -> bool {
        if self.source[self.pos..].starts_with(s) {
            self.pos += s.len();
            true
        } else {
            // Advance past whatever partial match there is
            while self.pos < self.source.len() && self.bytes[self.pos].is_ascii_alphabetic() {
                self.pos += 1;
            }
            false
        }
    }

    fn starts_with(&self, prefix: &[u8]) -> bool {
        self.bytes.get(self.pos..).is_some_and(|s| s.starts_with(prefix))
    }

    fn consume_digits(&mut self) {
        while self.pos < self.source.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
    }

    fn error(&mut self, message: &str, start: usize, end: usize) {
        self.errors.push(
            OxcDiagnostic::error(message.to_string())
                .with_label(Span::new(start as u32, end as u32)),
        );
    }
}
