# `ep-pin-toggle`

An implementation of the `EmbeddedProfiler` trait from [`embedded-profiling`].
Simply toggles the given GPIO rather than using a timer to profile. Useful when
you want to quickly profile a function using a logic analyzer or oscilloscope
in a resource constrained target.

## [Documentation](https://docs.rs/ep-pin-toggle/)

[`embedded-profiling`]: https://docs.rs/embedded-profiling

## Example Usage

An example usage can be found in [`embedded-profiling-examples`](https://github.com/TDHolmes/embedded-profiling).