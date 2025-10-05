//! Raw FFI bindings to nanopb (vendored from the firedancer repo)

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream() {
        let mut buffer = [0u8; 64];
        let stream = unsafe { pb_ostream_from_buffer(buffer.as_mut_ptr(), buffer.len()) };
        assert_eq!(stream.max_size, 64);
        assert_eq!(stream.bytes_written, 0);
    }

    #[test]
    fn test_istream() {
        let buffer = [0u8; 64];
        let stream = unsafe { pb_istream_from_buffer(buffer.as_ptr(), buffer.len()) };
        assert_eq!(stream.bytes_left, 64);
    }

    #[test]
    fn test_wiretypes() {
        assert_eq!(pb_wire_type_t_PB_WT_VARINT, 0);
        assert_eq!(pb_wire_type_t_PB_WT_64BIT, 1);
        assert_eq!(pb_wire_type_t_PB_WT_STRING, 2);
        assert_eq!(pb_wire_type_t_PB_WT_32BIT, 5);
    }
}
