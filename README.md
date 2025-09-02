# firedancer-rs

A collection of FFI bindings for [Firedancer](https://github.com/firedancer-io/firedancer). 

## Intention

While every crate in this repo could be used as-is in a given application, it's probably not a great idea unless you really know what you're doing. They come with all the benefits and drawbacks of performant, low-level APIs -- meaning that you'll likely blow your leg off if you aren't careful.

Rather, the crates in this repo are better served as components of rigorously tested higher-level crates that provide abstractions on top of the APIs provided here. 

## Non-Goals

- Bindings for every single firedancer module
- Documentation of everything
- High-level abstractions

## Progress

Currently, bindings are completed for a few components of [`utils`](https://github.com/firedancer-io/firedancer/blob/main/src/util/fd_util.h):

- `net`
- `bits`
- `log`
- `math`
- `env`
- `tpool` (partial)

### TODO

#### `utils`
- `tmpl`
- `alloc`
- `valloc`
- `scratch`
- `spad`
- `tile`
- `wksp`
- `shmem`
- `checkpt`

#### `waltz`
- `aio`
- `http`
- `quic`
- `grpc`
- `ebpf`
- `xdp`

### `funk`

#### `disco`
- `bundle`
- `store`

#### `ballet`
- `ed25519`
- `block`
- `txn`
- `shred`
- `base58`
- `base64`
- `sha1`
- `sha256`
- `sha512`
- `keccak256`
- `blake3`
- `json`
- `toml`
- `hex`
- `zstd`
- `bigint`

## License

[Apache 2.0](LICENSE)

## Acknowledgments

This project builds upon the work of Jump Crypto's [Firedancer](https://github.com/firedancer-io/firedancer) client, natively written in C. Give them your thanks and a star for their amazing work.
