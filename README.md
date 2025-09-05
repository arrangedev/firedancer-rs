# firedancer-rs

A collection of FFI bindings for [Firedancer](https://github.com/firedancer-io/firedancer). 

## Intention

While every crate in this repo could be used as-is in a given application, it's probably not a great idea unless you really know what you're doing. They come with all the benefits and drawbacks of performant, low-level APIs -- meaning that you'll likely blow your leg off if you aren't careful.

Rather, the crates in this repo are better served as components of rigorously tested higher-level crates that provide abstractions on top of the APIs provided here. 

## Non-Goals

- Bindings for every single firedancer module
- Documentation of everything
- High-level abstractions

## Status

| Module | Crate | macOS ARM64 | x86_64 Linux |
|--------|-------|-------------|--------------|
| **Ballet** |
| ed25519 | `fd_ed25519_sys` | √ | √ |
| sha256 | `fd_sha256_sys` | √ | √ |
| sha512 | `fd_sha512_sys` | √ | √ |
| keccak256 | `fd_keccak256_sys` | √ | √ |
| base64 | `fd_base64_sys` | √ | √ | |
| sbpf | `fd_sbpf_sys` | √ | √ |
| json | `fd_json_sys` | √ | √ | |
| toml | `fd_toml_sys` | √ | √ | |
| txn | `fd_txn_sys` | √ | √ |
| shred | `fd_shred_sys` | √ | √ |
| nanopb | `fd_nanopb_sys` | √ | √ |
| zstd | `fd_zstd_sys` | √ | x |
| **Tango** |
| mcache | `fd_mcache_sys` | √ | √ |
| dcache | `fd_dcache_sys` | √ | √ |
| tcache | `fd_tcache_sys` | √ | √ |
| **Util** |
| net | `fd_net_sys` | √ | √ |
| math | `fd_math_sys` | √ | √ |
| bits | `fd_bits_sys` | √ | √ |
| log | `fd_log_sys` | √ | √ |
| env | `fd_env_sys` | √ | √ |
| checkpt | `fd_checkpt_sys` | √ | x |
| valloc | `fd_valloc_sys` | √ | x |
| spad | `fd_spad_sys` | √ | x |
| tmpl | `fd_tmpl_sys` | √ | x |
| wksp | `fd_wksp_sys` | √ | x |
| tile | `fd_tile_sys` | √ | x |
| tpool | `fd_tpool_sys` | √ | x |
| scratch | `fd_scratch_sys` | √ | x |
| shmem | `fd_shmem_sys` | √ | x |

## License

[Apache 2.0](LICENSE)

## Acknowledgments

This project builds upon the work of Jump Crypto's [Firedancer](https://github.com/firedancer-io/firedancer) client, natively written in C. Give them your thanks and a star for their amazing work.
