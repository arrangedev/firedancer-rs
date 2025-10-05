//! Raw FFI bindings for `/util/checkpt`

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bindings_exist() {
        assert_eq!(FD_CHECKPT_SUCCESS, 0);
        assert!(FD_CHECKPT_ERR_INVAL < 0);
        assert!(FD_CHECKPT_ERR_UNSUP < 0);
        assert!(FD_CHECKPT_ERR_IO < 0);
        assert!(FD_CHECKPT_ERR_COMP < 0);
        assert_eq!(FD_CHECKPT_FRAME_STYLE_RAW, 1);
        assert_eq!(FD_CHECKPT_FRAME_STYLE_LZ4, 2);
        assert!(FD_CHECKPT_META_MAX > 0);
        assert!(FD_CHECKPT_WBUF_MIN > 0);
        assert!(FD_RESTORE_RBUF_MIN > 0);
    }

    #[test]
    fn test_framestyle_support() {
        unsafe {
            assert_eq!(
                fd_checkpt_frame_style_is_supported(FD_CHECKPT_FRAME_STYLE_RAW as i32),
                1
            );

            let lz4_supported =
                fd_checkpt_frame_style_is_supported(FD_CHECKPT_FRAME_STYLE_LZ4 as i32);

            assert!(lz4_supported == 0 || lz4_supported == 1);
            assert_eq!(fd_checkpt_frame_style_is_supported(999), 0);
        }
    }

    #[test]
    fn test_strerror() {
        unsafe {
            let success_msg = fd_checkpt_strerror(FD_CHECKPT_SUCCESS as i32);
            assert!(!success_msg.is_null());

            let inval_msg = fd_checkpt_strerror(FD_CHECKPT_ERR_INVAL);
            assert!(!inval_msg.is_null());

            let unknown_msg = fd_checkpt_strerror(-999);
            assert!(!unknown_msg.is_null());
        }
    }

    #[test]
    fn test_sizes() {
        assert!(core::mem::size_of::<fd_checkpt_private>() > 0);
        assert!(core::mem::size_of::<fd_restore_private>() > 0);
        assert!(core::mem::align_of::<fd_checkpt_private>() > 0);
        assert!(core::mem::align_of::<fd_restore_private>() > 0);
    }
}
