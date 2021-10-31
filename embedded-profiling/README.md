# Embedded-Profiling

A lightweight framework for profiling functions, geared towards
`no-std` embedded environments.

## [Documentation](https://docs.rs/embedded-profiling/)

## Usage

Initialization is very similar
to how the `log` crate is initialized. By default, there is a
no-op profiler that does nothing until you call `set_profiler`.
Once your profiler has been installed, your profiling
functionality will be in use.

You can manually start & end your snapshot:
```rust
let start = embedded_profiling::start_snapshot();
// (...) some expensive computation
let snapshot = embedded_profiling::end_snapshot(start, "name-of-computation");
// Optionally, log it
embedded_profiling::log_snapshot(&snapshot);
```

Or profile some code in a closure:
```rust
embedded_profiling::profile("profile println", || {
    println!("profiling this closure");
});
```

## With a Procedural Macro

With the `proc-macros` feature enabled, you can simply annotate
the target function with the procedural macro `profile_function`.
Note that you must first set your profiler with the`set_profiler`
function.
```rust
#[embedded_profiling::profile_function]
fn my_long_running_function() {
    println!("Hello, world!");
}
```

## Example Project & DWT Timer Implementation

A working example program on a [`feather_m4`] development board is provided
in the [`embedded-profiling` github repo](https://github.com/TDHolmes/embedded-profiling).
The `embedded-profiling` library also provides a basic timer implementation
using the DWT/systick functionality found in most Cortex-M microcontrollers.
The implementation is heavily inspired (read: copied and lightly modified)
from the `RTIC` [`dwt-systick-monotonic`](https://github.com/rtic-rs/dwt-systick-monotonic) crate.

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


[`feather_m4`]: https://www.adafruit.com/product/3857
