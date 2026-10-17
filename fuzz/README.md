# Fuzzing

Requires nightly Rust and `cargo install cargo-fuzz`. Run from the repository root:

```sh
cargo +nightly fuzz run parse -- -dict=fuzz/dict.txt fuzz/seeds
cargo +nightly fuzz run roundtrip -- -dict=fuzz/dict.txt fuzz/seeds
```

- `parse`: arbitrary input must never panic in parsing, validation,
  `media_segments`, writing, or variable substitution.
- `roundtrip`: a parsed playlist must write to text that parses again and writes
  identically.

`fuzz/seeds` holds hand-written starting inputs; the generated `corpus` and any
`artifacts` (crashes) are not committed. Turn a crash into a regression test in
`src/m3u8/tests/lib_tests.rs`.
