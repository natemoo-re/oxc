use std::borrow::Cow;

use oxc_span::Span;

/// A parsed JSON value with source span information.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue<'a> {
    Object(JsonObject<'a>),
    Array(JsonArray<'a>),
    String(JsonString<'a>),
    Number(JsonNumber<'a>),
    Bool(JsonBool),
    Null(Span),
}

impl JsonValue<'_> {
    pub fn span(&self) -> Span {
        match self {
            Self::Object(o) => o.span,
            Self::Array(a) => a.span,
            Self::String(s) => s.span,
            Self::Number(n) => n.span,
            Self::Bool(b) => b.span,
            Self::Null(span) => *span,
        }
    }

    pub fn as_object(&self) -> Option<&JsonObject<'_>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&JsonArray<'_>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(&s.value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(b.value),
            _ => None,
        }
    }
}

/// A JSON object: `{ "key": value, ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct JsonObject<'a> {
    pub span: Span,
    pub properties: Vec<JsonProperty<'a>>,
}

impl<'a> JsonObject<'a> {
    /// Get a property by key name.
    pub fn get(&self, key: &str) -> Option<&JsonProperty<'a>> {
        self.properties.iter().find(|p| p.key.value.as_ref() == key)
    }

    /// Get the value of a property by key name.
    pub fn get_value(&self, key: &str) -> Option<&JsonValue<'a>> {
        self.get(key).map(|p| &p.value)
    }
}

/// A JSON array: `[value, ...]`
#[derive(Debug, Clone, PartialEq)]
pub struct JsonArray<'a> {
    pub span: Span,
    pub elements: Vec<JsonValue<'a>>,
}

/// A key-value pair in a JSON object.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonProperty<'a> {
    /// Span covering the entire property (key + colon + value).
    pub span: Span,
    pub key: JsonString<'a>,
    pub value: JsonValue<'a>,
}

/// A JSON string value with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonString<'a> {
    /// Span covering the entire string including quotes.
    pub span: Span,
    /// The raw string content between quotes (preserves escape sequences as written).
    pub raw: &'a str,
    /// The decoded string value (escape sequences resolved).
    /// Borrows from source when no escape sequences are present.
    pub value: Cow<'a, str>,
}

/// A JSON number value with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonNumber<'a> {
    /// Span covering the number literal.
    pub span: Span,
    /// The raw number text.
    pub raw: &'a str,
}

impl JsonNumber<'_> {
    pub fn as_f64(&self) -> Option<f64> {
        self.raw.parse().ok()
    }
}

/// A JSON boolean value with its source span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonBool {
    pub span: Span,
    pub value: bool,
}

/// The kind of a JSONC comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonCommentKind {
    /// `// ...`
    Line,
    /// `/* ... */`
    Block,
}

/// A JSONC comment with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonComment<'a> {
    pub span: Span,
    pub value: &'a str,
    pub kind: JsonCommentKind,
}
