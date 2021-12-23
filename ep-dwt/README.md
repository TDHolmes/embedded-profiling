# `ep-dwt`

An implementation of the `EmbeddedProfiler` trait from [`embedded-profiling`] utilizing
the Data Watchpoint and Tracing (DWT) unit.
Derived from the RTIC monotonic implementation in
[`dwt_systick_monotonic`](https://docs.rs/dwt-systick-monotonic/)

## [Documentation](https://docs.rs/ep-dwt/)

[`embedded-profiling`]: https://docs.rs/embedded-profiling

## Example Usage

An example usage can be found in [`embedded-profiling-examples`](https://github.com/TDHolmes/embedded-profiling).

## Minimum Supported Rust Version (MSRV)

This crate is guaranteed to compile on stable Rust 1.57 and up. It might compile with older versions but that may change in any new patch release.

## License

This code is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
