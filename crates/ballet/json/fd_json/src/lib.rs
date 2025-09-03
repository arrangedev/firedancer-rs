//! Safe Rust API for Firedancer JSON utilities
//!
//! This crate provides safe abstractions over the raw FFI bindings in `fd_json_sys`.
//! The JSON implementation is based on the high-performance cJSON library and provides
//! complete JSON parsing, generation, and manipulation capabilities.
//!
//! ## Features
//!
//! - **High performance**: Built on the optimized cJSON library
//! - **Complete JSON support**: All JSON types including null, bool, number, string, array, object
//! - **Safe API**: All unsafe operations are encapsulated with proper error handling
//! - **Memory management**: Automatic cleanup with RAII patterns
//! - **Parsing and generation**: Parse JSON from strings and generate JSON strings
//! - **Manipulation**: Create, modify, and query JSON structures
//! - **Error handling**: Comprehensive error types with descriptive messages
//!
//! ## Usage
//!
//! ### Basic parsing and generation
//!
//! ```rust
//! use fd_json::{JsonValue, parse, to_string};
//!
//! let json_str = r#"{"name": "test", "value": 42, "active": true}"#;
//! let parsed = parse(json_str).unwrap();
//!
//! if let JsonValue::Object(ref obj) = parsed {
//!     let name = obj.get("name").and_then(|v| v.as_str()).unwrap();
//!     let value = obj.get("value").and_then(|v| v.as_f64()).unwrap();
//!     println!("Name: {}, Value: {}", name, value);
//! }
//!
//! let generated = to_string(&parsed).unwrap();
//! println!("Generated: {}", generated);
//! ```
//!
//! ### Building JSON structures
//!
//! ```rust
//! use fd_json::{json, JsonValue, JsonObject, JsonArray};
//!
//! let json_value = json!({
//!     "name": "example",
//!     "count": 10,
//!     "enabled": true,
//!     "items": ["first", "second"]
//! });
//!
//! let mut obj = JsonObject::new();
//! obj.insert("name".to_string(), JsonValue::String("example".to_string()));
//! obj.insert("count".to_string(), JsonValue::Number(10.0));
//! obj.insert("enabled".to_string(), JsonValue::Bool(true));
//!
//! let mut arr = JsonArray::new();
//! arr.push(JsonValue::String("first".to_string()));
//! arr.push(JsonValue::String("second".to_string()));
//! obj.insert("items".to_string(), JsonValue::Array(arr));
//!
//! let json_value = JsonValue::Object(obj);
//! let json_string = fd_json::to_string(&json_value).unwrap();
//! ```

use fd_json_sys as sys;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    /// JSON parsing failed
    ParseError(String),
    /// Invalid JSON structure
    InvalidStructure(String),
    /// Memory allocation failed
    MemoryError,
    /// Invalid UTF-8 sequence
    InvalidUtf8,
    /// Key not found in object
    KeyNotFound(String),
    /// Index out of bounds in array
    IndexOutOfBounds(usize),
    /// Type mismatch when accessing value
    TypeMismatch(String),
    /// Invalid input parameters
    InvalidInput(String),
    /// JSON generation failed
    GenerationError(String),
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonError::ParseError(msg) => write!(f, "JSON parse error: {}", msg),
            JsonError::InvalidStructure(msg) => write!(f, "Invalid JSON structure: {}", msg),
            JsonError::MemoryError => write!(f, "Memory allocation failed"),
            JsonError::InvalidUtf8 => write!(f, "Invalid UTF-8 sequence"),
            JsonError::KeyNotFound(key) => write!(f, "Key '{}' not found", key),
            JsonError::IndexOutOfBounds(index) => write!(f, "Index {} out of bounds", index),
            JsonError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            JsonError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            JsonError::GenerationError(msg) => write!(f, "JSON generation error: {}", msg),
        }
    }
}

impl std::error::Error for JsonError {}

pub type JsonObject = HashMap<String, JsonValue>;
pub type JsonArray = Vec<JsonValue>;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(JsonArray),
    Object(JsonObject),
}

impl JsonValue {
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, JsonValue::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, JsonValue::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, JsonValue::String(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, JsonValue::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, JsonValue::Object(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&JsonArray> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut JsonArray> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&JsonObject> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut JsonObject> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object()?.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut JsonValue> {
        self.as_object_mut()?.get_mut(key)
    }

    pub fn get_index(&self, index: usize) -> Option<&JsonValue> {
        self.as_array()?.get(index)
    }

    pub fn get_index_mut(&mut self, index: usize) -> Option<&mut JsonValue> {
        self.as_array_mut()?.get_mut(index)
    }
}

/// Convert cJSON pointer to JsonValue
unsafe fn cjson_to_json_value(cjson: *mut sys::cJSON) -> Result<JsonValue, JsonError> {
    if cjson.is_null() {
        return Ok(JsonValue::Null);
    }

    if sys::cJSON_IsNull(cjson) != 0 {
        Ok(JsonValue::Null)
    } else if sys::cJSON_IsBool(cjson) != 0 {
        Ok(JsonValue::Bool(sys::cJSON_IsTrue(cjson) != 0))
    } else if sys::cJSON_IsNumber(cjson) != 0 {
        Ok(JsonValue::Number(sys::cJSON_GetNumberValue(cjson)))
    } else if sys::cJSON_IsString(cjson) != 0 {
        let c_str = sys::cJSON_GetStringValue(cjson);
        if c_str.is_null() {
            return Err(JsonError::InvalidStructure(
                "String value is null".to_string(),
            ));
        }
        let rust_str = CStr::from_ptr(c_str)
            .to_str()
            .map_err(|_| JsonError::InvalidUtf8)?;
        Ok(JsonValue::String(rust_str.to_string()))
    } else if sys::cJSON_IsArray(cjson) != 0 {
        let mut array = JsonArray::new();
        let size = sys::cJSON_GetArraySize(cjson);
        for i in 0..size {
            let item = sys::cJSON_GetArrayItem(cjson, i);
            array.push(cjson_to_json_value(item)?);
        }
        Ok(JsonValue::Array(array))
    } else if sys::cJSON_IsObject(cjson) != 0 {
        let mut object = JsonObject::new();
        let mut item = (*cjson).child;
        while !item.is_null() {
            let key_ptr = (*item).string;
            if key_ptr.is_null() {
                return Err(JsonError::InvalidStructure(
                    "Object key is null".to_string(),
                ));
            }
            let key = CStr::from_ptr(key_ptr)
                .to_str()
                .map_err(|_| JsonError::InvalidUtf8)?;
            let value = cjson_to_json_value(item)?;
            object.insert(key.to_string(), value);
            item = (*item).next;
        }
        Ok(JsonValue::Object(object))
    } else {
        Err(JsonError::InvalidStructure("Unknown JSON type".to_string()))
    }
}

/// Convert JsonValue to cJSON pointer
unsafe fn json_value_to_cjson(value: &JsonValue) -> Result<*mut sys::cJSON, JsonError> {
    match value {
        JsonValue::Null => {
            let cjson = sys::cJSON_CreateNull();
            if cjson.is_null() {
                Err(JsonError::MemoryError)
            } else {
                Ok(cjson)
            }
        }
        JsonValue::Bool(b) => {
            let cjson = if *b {
                sys::cJSON_CreateTrue()
            } else {
                sys::cJSON_CreateFalse()
            };
            if cjson.is_null() {
                Err(JsonError::MemoryError)
            } else {
                Ok(cjson)
            }
        }
        JsonValue::Number(n) => {
            let cjson = sys::cJSON_CreateNumber(*n);
            if cjson.is_null() {
                Err(JsonError::MemoryError)
            } else {
                Ok(cjson)
            }
        }
        JsonValue::String(s) => {
            let c_string = CString::new(s.as_str())
                .map_err(|_| JsonError::InvalidInput("String contains null byte".to_string()))?;
            let cjson = sys::cJSON_CreateString(c_string.as_ptr());
            if cjson.is_null() {
                Err(JsonError::MemoryError)
            } else {
                Ok(cjson)
            }
        }
        JsonValue::Array(arr) => {
            let cjson = sys::cJSON_CreateArray();
            if cjson.is_null() {
                return Err(JsonError::MemoryError);
            }

            for item in arr {
                let cjson_item = json_value_to_cjson(item)?;
                if sys::cJSON_AddItemToArray(cjson, cjson_item) == 0 {
                    sys::cJSON_Delete(cjson_item);
                    sys::cJSON_Delete(cjson);
                    return Err(JsonError::MemoryError);
                }
            }
            Ok(cjson)
        }
        JsonValue::Object(obj) => {
            let cjson = sys::cJSON_CreateObject();
            if cjson.is_null() {
                return Err(JsonError::MemoryError);
            }

            for (key, value) in obj {
                let c_key = CString::new(key.as_str())
                    .map_err(|_| JsonError::InvalidInput("Key contains null byte".to_string()))?;
                let cjson_value = json_value_to_cjson(value)?;
                if sys::cJSON_AddItemToObject(cjson, c_key.as_ptr(), cjson_value) == 0 {
                    sys::cJSON_Delete(cjson_value);
                    sys::cJSON_Delete(cjson);
                    return Err(JsonError::MemoryError);
                }
            }
            Ok(cjson)
        }
    }
}

/// Parse a JSON string into JsonValue
pub fn parse(json_str: &str) -> Result<JsonValue, JsonError> {
    let c_string = CString::new(json_str)
        .map_err(|_| JsonError::InvalidInput("JSON string contains null byte".to_string()))?;

    unsafe {
        let cjson = sys::cJSON_Parse(c_string.as_ptr());
        if cjson.is_null() {
            let error_ptr = sys::cJSON_GetErrorPtr();
            let error_msg = if error_ptr.is_null() {
                "Unknown parse error".to_string()
            } else {
                let error_offset = error_ptr.offset_from(c_string.as_ptr()) as usize;
                format!("Parse error at position {}", error_offset)
            };
            return Err(JsonError::ParseError(error_msg));
        }

        let result = cjson_to_json_value(cjson);
        sys::cJSON_Delete(cjson);
        result
    }
}

/// Convert JsonValue to a JSON string
pub fn to_string(value: &JsonValue) -> Result<String, JsonError> {
    unsafe {
        let cjson = json_value_to_cjson(value)?;
        let c_str = sys::cJSON_Print(cjson);
        sys::cJSON_Delete(cjson);

        if c_str.is_null() {
            return Err(JsonError::GenerationError(
                "Failed to generate JSON string".to_string(),
            ));
        }

        let rust_str = CStr::from_ptr(c_str)
            .to_str()
            .map_err(|_| JsonError::InvalidUtf8)?
            .to_string();

        sys::cJSON_free(c_str as *mut std::ffi::c_void);
        Ok(rust_str)
    }
}

/// Convert JsonValue to a compacted JSON string (no formatting)
pub fn to_string_compact(value: &JsonValue) -> Result<String, JsonError> {
    unsafe {
        let cjson = json_value_to_cjson(value)?;
        let c_str = sys::cJSON_PrintUnformatted(cjson);
        sys::cJSON_Delete(cjson);

        if c_str.is_null() {
            return Err(JsonError::GenerationError(
                "Failed to generate JSON string".to_string(),
            ));
        }

        let rust_str = CStr::from_ptr(c_str)
            .to_str()
            .map_err(|_| JsonError::InvalidUtf8)?
            .to_string();

        sys::cJSON_free(c_str as *mut std::ffi::c_void);
        Ok(rust_str)
    }
}

impl JsonValue {
    pub fn null() -> Self {
        JsonValue::Null
    }

    pub fn bool(value: bool) -> Self {
        JsonValue::Bool(value)
    }

    pub fn number(value: f64) -> Self {
        JsonValue::Number(value)
    }

    pub fn number_i64(value: i64) -> Self {
        JsonValue::Number(value as f64)
    }

    pub fn string<S: Into<String>>(value: S) -> Self {
        JsonValue::String(value.into())
    }

    pub fn array() -> Self {
        JsonValue::Array(Vec::new())
    }

    pub fn array_from_vec(values: Vec<JsonValue>) -> Self {
        JsonValue::Array(values)
    }

    pub fn object() -> Self {
        JsonValue::Object(HashMap::new())
    }

    pub fn object_from_map(map: HashMap<String, JsonValue>) -> Self {
        JsonValue::Object(map)
    }
}

pub mod utils {
    use super::*;

    pub fn validate(json_str: &str) -> bool {
        parse(json_str).is_ok()
    }

    pub fn version() -> String {
        unsafe {
            let version_ptr = sys::cJSON_Version();
            if version_ptr.is_null() {
                "unknown".to_string()
            } else {
                CStr::from_ptr(version_ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub fn minify(json_str: &str) -> Result<String, JsonError> {
        let parsed = parse(json_str)?;
        to_string_compact(&parsed)
    }

    pub fn pretty_print(json_str: &str) -> Result<String, JsonError> {
        let parsed = parse(json_str)?;
        to_string(&parsed)
    }
}

/// Create a JSON value from a literal syntax
///
/// This macro allows you to construct JSON values using a syntax similar to JSON literals.
/// It supports all JSON types and nested structures.
///
/// # Examples
///
/// ```rust
/// use fd_json::{json, JsonValue};
///
/// // Objects
/// let obj = json!({
///     "name": "John",
///     "age": 30,
///     "active": true,
///     "balance": 1234.56,
///     "data": null
/// });
///
/// // Arrays
/// let arr = json!(["hello", "world", 42, true, null]);
///
/// // Nested structures
/// let nested = json!({
///     "users": [
///         {"id": 1, "name": "Alice"},
///         {"id": 2, "name": "Bob"}
///     ],
///     "metadata": {
///         "version": "1.0",
///         "count": 2
///     }
/// });
///
/// // Variables can be interpolated
/// let name = "dynamic";
/// let value = 42;
/// let dynamic = json!({
///     "name": name,
///     "value": value
/// });
/// ```
#[macro_export]
macro_rules! json {
    ({}) => {
        $crate::JsonValue::Object($crate::JsonObject::new())
    };

    ({ $($key:tt : $value:tt),+ $(,)? }) => {
        {
            let mut obj = $crate::JsonObject::new();
            $(
                obj.insert($crate::__json_key!($key), $crate::json!($value));
            )+
            $crate::JsonValue::Object(obj)
        }
    };

    ([]) => {
        $crate::JsonValue::Array($crate::JsonArray::new())
    };

    ([ $($element:tt),+ $(,)? ]) => {
        {
            let mut arr = $crate::JsonArray::new();
            $(
                arr.push($crate::json!($element));
            )+
            $crate::JsonValue::Array(arr)
        }
    };

    (null) => {
        $crate::JsonValue::Null
    };

    (true) => {
        $crate::JsonValue::Bool(true)
    };

    (false) => {
        $crate::JsonValue::Bool(false)
    };

    ($string:literal) => {
        $crate::JsonValue::String($string.to_string())
    };

    ($expr:expr) => {
        $crate::__json_value!($expr)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __json_key {
    ($key:literal) => {
        $key.to_string()
    };
    ($key:expr) => {
        $key.to_string()
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __json_value {
    ($expr:expr) => {
        $crate::__json_convert($expr)
    };
}

#[doc(hidden)]
pub fn __json_convert<T>(value: T) -> JsonValue
where
    T: Into<JsonValue>,
{
    value.into()
}

/// Parse JSON from a string literal
///
/// # Example
///
/// ```rust
/// use fd_json::json_parse;
///
/// let value = json_parse!(r#"{"name": "test", "value": 42}"#);
/// ```
#[macro_export]
macro_rules! json_parse {
    ($json_str:literal) => {
        $crate::parse($json_str).expect("Invalid JSON literal")
    };
}

/// Extract a value from a JSON path
///
/// # Example
///
/// ```rust
/// use fd_json::{json, json_get};
///
/// let data = json!({
///     "user": {
///         "name": "John",
///         "settings": {
///             "theme": "dark"
///         }
///     },
///     "items": [1, 2, 3]
/// });
///
/// let name: Option<&str> = json_get!(data, "user", "name").and_then(|v| v.as_str());
/// let theme: Option<&str> = json_get!(data, "user", "settings", "theme").and_then(|v| v.as_str());
/// let first_item: Option<f64> = json_get_index!(data, "items", 0).and_then(|v| v.as_f64());
/// ```
#[macro_export]
macro_rules! json_get {
    ($json:expr, $key:expr) => {
        $json.get($key)
    };

    ($json:expr, $key:expr, $($rest:expr),+) => {
        $json.get($key).and_then(|v| $crate::json_get!(v, $($rest),+))
    };
}

/// Extract a value from a JSON array by index
#[macro_export]
macro_rules! json_get_index {
    ($json:expr, $index:expr) => {
        $json.get_index($index)
    };

    ($json:expr, $key:expr, $index:expr) => {
        $json.get($key).and_then(|v| v.get_index($index))
    };

    ($json:expr, $key:expr, $($rest:expr),+) => {
        $json.get($key).and_then(|v| $crate::json_get_index!(v, $($rest),+))
    };
}

/// Assert that a JSON value matches an expected pattern
///
/// # Example
///
/// ```rust
/// use fd_json::{json, json_assert};
///
/// let data = json!({
///     "status": "success",
///     "data": {
///         "count": 42
///     }
/// });
///
/// json_assert!(data, {
///     "status": "success",
///     "data": {
///         "count": 42
///     }
/// });
/// ```
#[macro_export]
macro_rules! json_assert {
    ($actual:expr, $expected:tt) => {
        assert_eq!($actual, $crate::json!($expected));
    };
}

/// Create a JSON array from a list of expressions
///
/// # Example
///
/// ```rust
/// use fd_json::json_array;
///
/// let numbers = vec![1, 2, 3];
/// let arr = json_array![1, "hello", true, numbers];
/// ```
#[macro_export]
macro_rules! json_array {
    ($($element:expr),* $(,)?) => {
        $crate::JsonValue::Array(vec![$($crate::__json_convert($element)),*])
    };
}

/// Create a JSON object from key-value pairs
///
/// # Example
///
/// ```rust
/// use fd_json::json_object;
///
/// let name = "John";
/// let age = 30;
/// let obj = json_object! {
///     "name" => name,
///     "age" => age,
///     "active" => true
/// };
/// ```
#[macro_export]
macro_rules! json_object {
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            let mut obj = $crate::JsonObject::new();
            $(
                obj.insert($key.to_string(), $crate::__json_convert($value));
            )*
            $crate::JsonValue::Object(obj)
        }
    };
}

/// Update a JSON object by adding or modifying fields
///
/// # Example
///
/// ```rust
/// use fd_json::{json, json_update};
///
/// let mut data = json!({"name": "John", "age": 30});
/// json_update!(data, {
///     "age": 31,
///     "city": "New York"
/// });
/// ```
#[macro_export]
macro_rules! json_update {
    ($json:expr, { $($key:tt : $value:tt),+ $(,)? }) => {
        {
            if let Some(obj) = $json.as_object_mut() {
                $(
                    obj.insert($crate::__json_key!($key), $crate::json!($value));
                )+
            }
        }
    };
}

/// Merge two JSON objects
///
/// # Example
///
/// ```rust
/// use fd_json::{json, json_merge};
///
/// let mut base = json!({"name": "John", "age": 30});
/// let update = json!({"age": 31, "city": "New York"});
/// json_merge!(base, update);
/// ```
#[macro_export]
macro_rules! json_merge {
    ($target:expr, $source:expr) => {{
        if let (Some(target_obj), Some(source_obj)) = ($target.as_object_mut(), $source.as_object())
        {
            for (key, value) in source_obj {
                target_obj.insert(key.clone(), value.clone());
            }
        }
    }};
}

/// Pretty-print a JSON value
///
/// # Example
///
/// ```rust
/// use fd_json::{json, json_debug};
///
/// let data = json!({"name": "John", "items": [1, 2, 3]});
/// json_debug!(data);
/// json_debug!(data, "User data");
/// ```
#[macro_export]
macro_rules! json_debug {
    ($json:expr) => {
        match $crate::to_string(&$json) {
            Ok(s) => println!("{}", s),
            Err(e) => println!("JSON debug error: {}", e),
        }
    };

    ($json:expr, $label:expr) => {
        match $crate::to_string(&$json) {
            Ok(s) => println!("{}: {}", $label, s),
            Err(e) => println!("{} - JSON debug error: {}", $label, e),
        }
    };
}

/// Convert a JSON value to a compact string, panicking on error
///
/// # Example
///
/// ```rust
/// use fd_json::{json, json_stringify};
///
/// let data = json!({"name": "John"});
/// let json_str = json_stringify!(data);
/// ```
#[macro_export]
macro_rules! json_stringify {
    ($json:expr) => {
        $crate::to_string_compact(&$json).expect("Failed to stringify JSON")
    };
}

/// Parse a JSON string, panicking on error
///
/// # Example
///
/// ```rust
/// use fd_json::json_parse_str;
///
/// let json_str = r#"{"name": "John", "age": 30}"#;
/// let data = json_parse_str!(json_str);
/// ```
#[macro_export]
macro_rules! json_parse_str {
    ($json_str:expr) => {
        $crate::parse($json_str).expect("Failed to parse JSON")
    };
}

impl From<bool> for JsonValue {
    fn from(b: bool) -> Self {
        JsonValue::Bool(b)
    }
}

impl From<i8> for JsonValue {
    fn from(n: i8) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<i16> for JsonValue {
    fn from(n: i16) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<i32> for JsonValue {
    fn from(n: i32) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<i64> for JsonValue {
    fn from(n: i64) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<u8> for JsonValue {
    fn from(n: u8) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<u16> for JsonValue {
    fn from(n: u16) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<u32> for JsonValue {
    fn from(n: u32) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<u64> for JsonValue {
    fn from(n: u64) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<f32> for JsonValue {
    fn from(n: f32) -> Self {
        JsonValue::Number(n as f64)
    }
}

impl From<f64> for JsonValue {
    fn from(n: f64) -> Self {
        JsonValue::Number(n)
    }
}

impl From<&str> for JsonValue {
    fn from(s: &str) -> Self {
        JsonValue::String(s.to_string())
    }
}

impl From<String> for JsonValue {
    fn from(s: String) -> Self {
        JsonValue::String(s)
    }
}

impl From<Vec<JsonValue>> for JsonValue {
    fn from(arr: Vec<JsonValue>) -> Self {
        JsonValue::Array(arr)
    }
}

impl From<HashMap<String, JsonValue>> for JsonValue {
    fn from(obj: HashMap<String, JsonValue>) -> Self {
        JsonValue::Object(obj)
    }
}

impl<T> From<Option<T>> for JsonValue
where
    T: Into<JsonValue>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(value) => value.into(),
            None => JsonValue::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_object() {
        let json_str = r#"{"name": "test", "value": 42, "active": true}"#;
        let parsed = parse(json_str).unwrap();

        if let JsonValue::Object(obj) = parsed {
            assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("test"));
            assert_eq!(obj.get("value").and_then(|v| v.as_f64()), Some(42.0));
            assert_eq!(obj.get("active").and_then(|v| v.as_bool()), Some(true));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_array() {
        let json_str = r#"["first", "second", 123, true, null]"#;
        let parsed = parse(json_str).unwrap();

        if let JsonValue::Array(arr) = parsed {
            assert_eq!(arr.len(), 5);
            assert_eq!(arr[0].as_str(), Some("first"));
            assert_eq!(arr[1].as_str(), Some("second"));
            assert_eq!(arr[2].as_f64(), Some(123.0));
            assert_eq!(arr[3].as_bool(), Some(true));
            assert!(arr[4].is_null());
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_generate() {
        let mut obj = JsonObject::new();
        obj.insert("name".to_string(), JsonValue::String("test".to_string()));
        obj.insert("value".to_string(), JsonValue::Number(42.0));
        obj.insert("active".to_string(), JsonValue::Bool(true));

        let json_value = JsonValue::Object(obj);
        let json_str = to_string(&json_value).unwrap();

        let reparsed = parse(&json_str).unwrap();
        assert_eq!(json_value, reparsed);
    }

    #[test]
    fn test_nested() {
        let json_str = r#"{
            "user": {
                "name": "John",
                "age": 30,
                "hobbies": ["reading", "swimming"]
            },
            "settings": {
                "theme": "dark",
                "notifications": true
            }
        }"#;

        let parsed = parse(json_str).unwrap();

        let user_name = parsed
            .get("user")
            .and_then(|u| u.get("name"))
            .and_then(|n| n.as_str());
        assert_eq!(user_name, Some("John"));

        let hobbies = parsed
            .get("user")
            .and_then(|u| u.get("hobbies"))
            .and_then(|h| h.as_array());
        assert_eq!(hobbies.unwrap().len(), 2);
        assert_eq!(hobbies.unwrap()[0].as_str(), Some("reading"));
    }

    #[test]
    fn test_compact_pretty() {
        let obj = JsonValue::object_from_map({
            let mut map = HashMap::new();
            map.insert("key".to_string(), JsonValue::string("value"));
            map.insert("number".to_string(), JsonValue::number(42.0));
            map
        });

        let pretty = to_string(&obj).unwrap();
        let compact = to_string_compact(&obj).unwrap();

        assert!(compact.len() <= pretty.len());

        let parsed_pretty = parse(&pretty).unwrap();
        let parsed_compact = parse(&compact).unwrap();
        assert_eq!(parsed_pretty, parsed_compact);
    }

    #[test]
    fn test_error_handling() {
        let invalid_json = r#"{"invalid": json}"#;
        let result = parse(invalid_json);
        assert!(result.is_err());

        if let Err(JsonError::ParseError(_)) = result {
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_utils() {
        assert!(utils::validate(r#"{"valid": true}"#));
        assert!(!utils::validate(r#"{"invalid": json}"#));

        let version = utils::version();
        assert!(version.starts_with("1.7"));

        let minified = utils::minify(r#"{  "key"  :  "value"  }"#).unwrap();
        assert!(!minified.contains("  "));

        let pretty = utils::pretty_print(r#"{"key":"value"}"#).unwrap();
        assert!(pretty.contains('\n') || pretty.contains(' '));
    }

    #[test]
    fn test_roundtrip() {
        let test_cases = [
            r#"null"#,
            r#"true"#,
            r#"false"#,
            r#"42"#,
            r#"3.14159"#,
            r#""hello world""#,
            r#"[]"#,
            r#"[1, 2, 3]"#,
            r#"{}"#,
            r#"{"a": 1, "b": [2, 3], "c": {"d": 4}}"#,
        ];

        for case in &test_cases {
            let parsed = parse(case).unwrap();
            let generated = to_string_compact(&parsed).unwrap();
            let reparsed = parse(&generated).unwrap();
            assert_eq!(parsed, reparsed, "Round-trip failed for: {}", case);
        }
    }

    #[test]
    fn test_type_access() {
        let json_str =
            r#"{"str": "hello", "num": 42, "bool": true, "null": null, "arr": [1,2], "obj": {}}"#;
        let parsed = parse(json_str).unwrap();

        if let JsonValue::Object(obj) = parsed {
            assert!(obj["str"].is_string());
            assert!(obj["num"].is_number());
            assert!(obj["bool"].is_bool());
            assert!(obj["null"].is_null());
            assert!(obj["arr"].is_array());
            assert!(obj["obj"].is_object());
        }
    }

    #[test]
    fn test_json_macro_basic() {
        let null_val = json!(null);
        assert!(null_val.is_null());

        let bool_val = json!(true);
        assert_eq!(bool_val.as_bool(), Some(true));

        let number_val = json!(42);
        assert_eq!(number_val.as_f64(), Some(42.0));

        let string_val = json!("hello");
        assert_eq!(string_val.as_str(), Some("hello"));
    }

    #[test]
    fn test_json_macro_arrays() {
        let empty_arr = json!([]);
        assert!(empty_arr.is_array());
        assert_eq!(empty_arr.as_array().unwrap().len(), 0);

        let arr = json!([1, "hello", true, null]);
        let arr_ref = arr.as_array().unwrap();
        assert_eq!(arr_ref.len(), 4);
        assert_eq!(arr_ref[0].as_f64(), Some(1.0));
        assert_eq!(arr_ref[1].as_str(), Some("hello"));
        assert_eq!(arr_ref[2].as_bool(), Some(true));
        assert!(arr_ref[3].is_null());
    }

    #[test]
    fn test_json_macro_objects() {
        let empty_obj = json!({});
        assert!(empty_obj.is_object());
        assert_eq!(empty_obj.as_object().unwrap().len(), 0);

        let obj = json!({
            "name": "John",
            "age": 30,
            "active": true,
            "data": null
        });

        assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("John"));
        assert_eq!(obj.get("age").and_then(|v| v.as_f64()), Some(30.0));
        assert_eq!(obj.get("active").and_then(|v| v.as_bool()), Some(true));
        assert!(obj.get("data").unwrap().is_null());
    }

    #[test]
    fn test_json_macro_nested() {
        let nested = json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ],
            "metadata": {
                "version": "1.0",
                "count": 2
            }
        });

        let alice_name = nested
            .get("users")
            .and_then(|u| u.get_index(0))
            .and_then(|u| u.get("name"))
            .and_then(|n| n.as_str());
        assert_eq!(alice_name, Some("Alice"));

        let version = nested
            .get("metadata")
            .and_then(|m| m.get("version"))
            .and_then(|v| v.as_str());
        assert_eq!(version, Some("1.0"));
    }

    #[test]
    fn test_json_macro_variables() {
        let name = "dynamic";
        let age = 42;
        let active = true;

        let obj = json!({
            "name": name,
            "age": age,
            "active": active
        });

        assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("dynamic"));
        assert_eq!(obj.get("age").and_then(|v| v.as_f64()), Some(42.0));
        assert_eq!(obj.get("active").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn test_from_implementations() {
        let values: Vec<(JsonValue, JsonValue)> = vec![
            (JsonValue::from(42i32), json!(42)),
            (JsonValue::from(true), json!(true)),
            (JsonValue::from("hello"), json!("hello")),
            (JsonValue::from(String::from("world")), json!("world")),
            (JsonValue::from(Some(42)), json!(42)),
            (JsonValue::from(None::<i32>), json!(null)),
        ];

        for (actual, expected) in values {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_array_macro() {
        let arr = json_array![1, "hello", true];
        let expected = json!([1, "hello", true]);
        assert_eq!(arr, expected);

        let empty = json_array![];
        assert!(empty.is_array());
        assert_eq!(empty.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_object_macro() {
        let name = "John";
        let age = 30;

        let obj = json_object! {
            "name" => name,
            "age" => age,
            "active" => true
        };

        assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("John"));
        assert_eq!(obj.get("age").and_then(|v| v.as_f64()), Some(30.0));
        assert_eq!(obj.get("active").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn test_get_macro() {
        let data = json!({
            "user": {
                "name": "John",
                "settings": {
                    "theme": "dark"
                }
            },
            "items": [1, 2, 3]
        });

        let name = json_get!(data, "user", "name").and_then(|v| v.as_str());
        assert_eq!(name, Some("John"));

        let theme = json_get!(data, "user", "settings", "theme").and_then(|v| v.as_str());
        assert_eq!(theme, Some("dark"));

        let first_item = json_get_index!(data, "items", 0).and_then(|v| v.as_f64());
        assert_eq!(first_item, Some(1.0));

        let missing = json_get!(data, "missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_update_macro() {
        let mut data = json!({"name": "John", "age": 30});

        json_update!(data, {
            "age": 31,
            "city": "New York"
        });

        assert_eq!(data.get("name").and_then(|v| v.as_str()), Some("John"));
        assert_eq!(data.get("age").and_then(|v| v.as_f64()), Some(31.0));
        assert_eq!(data.get("city").and_then(|v| v.as_str()), Some("New York"));
    }

    #[test]
    fn test_merge_macro() {
        let mut base = json!({"name": "John", "age": 30});
        let update = json!({"age": 31, "city": "New York"});

        json_merge!(base, update);
        assert_eq!(base.get("name").and_then(|v| v.as_str()), Some("John"));
        assert_eq!(base.get("age").and_then(|v| v.as_f64()), Some(31.0));
        assert_eq!(base.get("city").and_then(|v| v.as_str()), Some("New York"));
    }

    #[test]
    fn test_assert_macro() {
        let data = json!({
            "status": "success",
            "count": 42
        });

        json_assert!(data, {
            "status": "success",
            "count": 42
        });
    }

    #[test]
    fn test_stringify_macro() {
        let data = json!({"name": "John"});
        let json_str = json_stringify!(data);
        assert!(json_str.contains("name"));
        assert!(json_str.contains("John"));
        assert!(!json_str.contains("  "));
    }

    #[test]
    fn test_parse_str_macro() {
        let json_str = r#"{"name": "John", "age": 30}"#;
        let data = json_parse_str!(json_str);

        assert_eq!(data.get("name").and_then(|v| v.as_str()), Some("John"));
        assert_eq!(data.get("age").and_then(|v| v.as_f64()), Some(30.0));
    }

    #[test]
    fn test_macro_combo() {
        let name = "Alice";
        let scores: Vec<JsonValue> = vec![95, 87, 92].into_iter().map(JsonValue::from).collect();

        let student = json!({
            "name": name,
            "scores": scores,
            "metadata": {
                "enrolled": true,
                "semester": "fall"
            }
        });

        let student_name = json_get!(student, "name").and_then(|v| v.as_str());
        assert_eq!(student_name, Some("Alice"));
        let enrolled = json_get!(student, "metadata", "enrolled").and_then(|v| v.as_bool());
        assert_eq!(enrolled, Some(true));
        let first_score = json_get_index!(student, "scores", 0).and_then(|v| v.as_f64());
        assert_eq!(first_score, Some(95.0));

        let json_str = json_stringify!(student);
        let parsed_back = json_parse_str!(&json_str);
        assert_eq!(student, parsed_back);
    }

    #[test]
    fn test_trailing_commas() {
        let obj = json!({
            "a": 1,
            "b": 2,
        });
        assert_eq!(obj.get("a").and_then(|v| v.as_f64()), Some(1.0));
        assert_eq!(obj.get("b").and_then(|v| v.as_f64()), Some(2.0));

        let arr = json!([1, 2, 3,]);
        assert_eq!(arr.as_array().unwrap().len(), 3);
    }
}
