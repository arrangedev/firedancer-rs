# firedancer-rs

A collection of FFI bindings for [Firedancer](https://github.com/firedancer-io/firedancer). 

## Goals

## Non-Goals

- Bindings for every firedancer module

## Progress

Currently, bindings are completed for a few components of [`utils`](https://github.com/firedancer-io/firedancer/blob/main/src/util/fd_util.h):

- `net`
- `bits`
- `log`
- `math`

TODO (utils):

- `env` 
- `tpool`
- `checkpt`
- `tmpl`
- `alloc`
- `shmem`

TODO (other):

- `tango`
- `waltz`
- `ballet`

## License

[Apache 2.0](LICENSE)

## Acknowledgments

This project builds upon the work of Jump Crypto's [Firedancer](https://github.com/firedancer-io/firedancer) client, natively written in C. Give them your thanks and a star for their amazing work.
