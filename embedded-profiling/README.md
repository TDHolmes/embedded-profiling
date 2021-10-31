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