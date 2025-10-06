#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_constants() {
        // Test that basic constants are available
        assert!(FD_TOPO_MAX_WKSPS > 0);
        assert!(FD_TOPO_MAX_LINKS > 0);
        assert!(FD_TOPO_MAX_TILES > 0);
        assert!(FD_TOPO_MAX_OBJS > 0);
    }

    #[test]
    fn test_struct_sizes() {
        // Ensure structs have reasonable sizes
        assert!(std::mem::size_of::<fd_topo_wksp_t>() > 0);
        assert!(std::mem::size_of::<fd_topo_link_t>() > 0);
        assert!(std::mem::size_of::<fd_topo_tile_t>() > 0);
        assert!(std::mem::size_of::<fd_topo_obj_t>() > 0);
        assert!(std::mem::size_of::<fd_topo_t>() > 0);
    }
}
