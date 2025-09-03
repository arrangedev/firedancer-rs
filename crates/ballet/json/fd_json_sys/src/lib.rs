//! Raw FFI bindings to Firedancer JSON utilities
//!
//! This crate provides low-level, unsafe bindings to the cJSON library used by Firedancer:
//! - High-performance JSON parsing and generation
//! - Complete cJSON API including parsing, printing, and manipulation
//! - Support for all JSON types: null, bool, number, string, array, object
//! - Memory management utilities
//! - Error handling and validation
//!
//! For a safe Rust API, consider using the higher-level `fd_json` wrapper crate.
//!
//! # Example
//!
//! ```rust,no_run
//! use fd_json_sys::*;
//! use std::ffi::{CStr, CString};
//!
//! unsafe {
//!     let json_str = CString::new(r#"{"name": "test", "value": 42}"#).unwrap();
//!     let json = cJSON_Parse(json_str.as_ptr());
//!     
//!     if !json.is_null() {
//!         let name_key = CString::new("name").unwrap();
//!         let name_item = cJSON_GetObjectItem(json, name_key.as_ptr());
//!         
//!         if !name_item.is_null() {
//!             let name_value = cJSON_GetStringValue(name_item);
//!             if !name_value.is_null() {
//!                 let name = CStr::from_ptr(name_value).to_string_lossy();
//!                 println!("Name: {}", name);
//!             }
//!         }
//!         
//!         cJSON_Delete(json);
//!     }
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    #[test]
    fn test_json_version() {
        unsafe {
            let version = cJSON_Version();
            assert!(!version.is_null());
            let version_str = CStr::from_ptr(version).to_string_lossy();
            assert!(version_str.starts_with("1.7"));
        }
    }

    #[test]
    fn test_json_parse_simple() {
        unsafe {
            let json_str = CString::new(r#"{"test": true}"#).unwrap();
            let json = cJSON_Parse(json_str.as_ptr());
            assert!(!json.is_null());

            assert_eq!(cJSON_IsObject(json), 1);

            let test_key = CString::new("test").unwrap();
            let test_item = cJSON_GetObjectItem(json, test_key.as_ptr());
            assert!(!test_item.is_null());
            assert_eq!(cJSON_IsTrue(test_item), 1);

            cJSON_Delete(json);
        }
    }

    #[test]
    fn test_json_create_and_print() {
        unsafe {
            let root = cJSON_CreateObject();
            assert!(!root.is_null());

            let name_key = CString::new("name").unwrap();
            let name_value = CString::new("test").unwrap();
            let name_item = cJSON_CreateString(name_value.as_ptr());
            assert_eq!(cJSON_AddItemToObject(root, name_key.as_ptr(), name_item), 1);

            let number_key = CString::new("value").unwrap();
            let number_item = cJSON_CreateNumber(42.0);
            assert_eq!(
                cJSON_AddItemToObject(root, number_key.as_ptr(), number_item),
                1
            );

            let json_str = cJSON_Print(root);
            assert!(!json_str.is_null());

            let result = CStr::from_ptr(json_str).to_string_lossy();
            assert!(result.contains("name"));
            assert!(result.contains("test"));
            assert!(result.contains("value"));
            assert!(result.contains("42"));

            cJSON_free(json_str as *mut std::ffi::c_void);
            cJSON_Delete(root);
        }
    }

    #[test]
    fn test_json_array() {
        unsafe {
            let array = cJSON_CreateArray();
            assert!(!array.is_null());

            let item1 = cJSON_CreateString(CString::new("first").unwrap().as_ptr());
            let item2 = cJSON_CreateString(CString::new("second").unwrap().as_ptr());
            let item3 = cJSON_CreateNumber(123.0);

            assert_eq!(cJSON_AddItemToArray(array, item1), 1);
            assert_eq!(cJSON_AddItemToArray(array, item2), 1);
            assert_eq!(cJSON_AddItemToArray(array, item3), 1);

            assert_eq!(cJSON_GetArraySize(array), 3);

            let first_item = cJSON_GetArrayItem(array, 0);
            assert!(!first_item.is_null());
            assert_eq!(cJSON_IsString(first_item), 1);

            let third_item = cJSON_GetArrayItem(array, 2);
            assert!(!third_item.is_null());
            assert_eq!(cJSON_IsNumber(third_item), 1);
            assert_eq!(cJSON_GetNumberValue(third_item), 123.0);

            cJSON_Delete(array);
        }
    }

    #[test]
    fn test_json_types() {
        unsafe {
            let null_item = cJSON_CreateNull();
            assert_eq!(cJSON_IsNull(null_item), 1);

            let true_item = cJSON_CreateTrue();
            assert_eq!(cJSON_IsTrue(true_item), 1);
            assert_eq!(cJSON_IsBool(true_item), 1);

            let false_item = cJSON_CreateFalse();
            assert_eq!(cJSON_IsFalse(false_item), 1);
            assert_eq!(cJSON_IsBool(false_item), 1);

            let number_item = cJSON_CreateNumber(3.14159);
            assert_eq!(cJSON_IsNumber(number_item), 1);
            assert!((cJSON_GetNumberValue(number_item) - 3.14159).abs() < 0.0001);

            let string_item = cJSON_CreateString(CString::new("hello").unwrap().as_ptr());
            assert_eq!(cJSON_IsString(string_item), 1);
            let string_value = cJSON_GetStringValue(string_item);
            assert!(!string_value.is_null());
            let string_result = CStr::from_ptr(string_value).to_string_lossy();
            assert_eq!(string_result, "hello");

            cJSON_Delete(null_item);
            cJSON_Delete(true_item);
            cJSON_Delete(false_item);
            cJSON_Delete(number_item);
            cJSON_Delete(string_item);
        }
    }

    #[test]
    fn test_json_parse_error() {
        unsafe {
            let invalid_json = CString::new(r#"{"invalid": json}"#).unwrap();
            let json = cJSON_Parse(invalid_json.as_ptr());
            assert!(json.is_null());

            let error_ptr = cJSON_GetErrorPtr();
            assert!(!error_ptr.is_null());
        }
    }

    #[test]
    fn test_json_object_operations() {
        unsafe {
            let obj = cJSON_CreateObject();
            assert!(!obj.is_null());

            let key = CString::new("testkey").unwrap();
            let value = cJSON_CreateString(CString::new("testvalue").unwrap().as_ptr());

            assert_eq!(cJSON_AddItemToObject(obj, key.as_ptr(), value), 1);
            assert_eq!(cJSON_HasObjectItem(obj, key.as_ptr()), 1);

            let retrieved = cJSON_GetObjectItem(obj, key.as_ptr());
            assert!(!retrieved.is_null());
            assert_eq!(cJSON_IsString(retrieved), 1);

            let retrieved_value = cJSON_GetStringValue(retrieved);
            assert!(!retrieved_value.is_null());
            let result = CStr::from_ptr(retrieved_value).to_string_lossy();
            assert_eq!(result, "testvalue");

            cJSON_Delete(obj);
        }
    }

    #[test]
    fn test_json_constants() {
        // Test that the constants are defined correctly
        assert_eq!(cJSON_Invalid, 0);
        assert_eq!(cJSON_False, 1);
        assert_eq!(cJSON_True, 2);
        assert_eq!(cJSON_NULL, 4);
        assert_eq!(cJSON_Number, 8);
        assert_eq!(cJSON_String, 16);
        assert_eq!(cJSON_Array, 32);
        assert_eq!(cJSON_Object, 64);
    }

    #[test]
    fn test_json_print_unformatted() {
        unsafe {
            let obj = cJSON_CreateObject();
            let key = CString::new("key").unwrap();
            let value = cJSON_CreateString(CString::new("value").unwrap().as_ptr());
            cJSON_AddItemToObject(obj, key.as_ptr(), value);

            let formatted = cJSON_Print(obj);
            let unformatted = cJSON_PrintUnformatted(obj);

            assert!(!formatted.is_null());
            assert!(!unformatted.is_null());

            let formatted_str = CStr::from_ptr(formatted).to_string_lossy();
            let unformatted_str = CStr::from_ptr(unformatted).to_string_lossy();

            // Unformatted should be shorter (no whitespace)
            assert!(unformatted_str.len() <= formatted_str.len());
            assert!(unformatted_str.contains("key"));
            assert!(unformatted_str.contains("value"));

            cJSON_free(formatted as *mut std::ffi::c_void);
            cJSON_free(unformatted as *mut std::ffi::c_void);
            cJSON_Delete(obj);
        }
    }
}
