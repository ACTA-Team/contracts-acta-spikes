# Fuzzing Setup

This document describes how to install, build, and run the `cargo-fuzz` targets
that live under `contracts/*/fuzz/`.

---

## Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust (stable) | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Rust (nightly) | nightly | `rustup toolchain install nightly` |
| cargo-fuzz | latest | `cargo install cargo-fuzz` |

Verify:

```sh
rustc --version
rustup run nightly rustc --version
cargo fuzz --version
```

---

## Repository layout

Each contract that has a fuzz suite contains a `fuzz/` sub-crate:

```
contracts/
  vc-issuer-registry/
    fuzz/
      Cargo.toml                         # Isolated fuzz workspace
      fuzz_targets/
        fuzz_issuer_metadata.rs          # Drives add_issuer / set_issuer_metadata
  vc-schema-registry/
    fuzz/
      Cargo.toml
      fuzz_targets/
        fuzz_schema_id.rs                # Drives schema_id derivation + registration
  vc-revocation-registry/
    fuzz/
      Cargo.toml
      fuzz_targets/
        fuzz_credential_id.rs            # Drives revoke / unrevoke with arbitrary IDs
```

Each `fuzz/` directory is its own Cargo workspace (`[workspace]`) so it is
excluded from the root workspace and never pulled into the normal `cargo build`.

---

## Running fuzz targets

### vc-issuer-registry — `fuzz_issuer_metadata`

Exercises `add_issuer` and `set_issuer_metadata` with arbitrary `did` / `url`
byte payloads. Asserts:

- Payloads ≤ 256 bytes are always accepted.
- Payloads > 256 bytes are always rejected.
- `is_issuer_allowed` is `true` immediately after a successful `add_issuer`.
- `get_issuer` returns the stored record without panicking.

```sh
cd contracts/vc-issuer-registry
cargo +nightly fuzz run fuzz_issuer_metadata --sanitizer none
```

To run for a bounded time (e.g. 60 seconds, as used in CI):

```sh
cargo +nightly fuzz run fuzz_issuer_metadata --sanitizer none -- -max_total_time=60
```

### vc-schema-registry — `fuzz_schema_id`

Exercises `register_schema` and `schema_id` with arbitrary `(name, version)`
inputs. Asserts:

- `schema_id()` is deterministic for the same triple.
- The ID from `register_schema` matches `schema_id()`.
- Duplicate registrations are rejected.

```sh
cd contracts/vc-schema-registry
cargo +nightly fuzz run fuzz_schema_id --sanitizer none
```

Bounded run:

```sh
cargo +nightly fuzz run fuzz_schema_id --sanitizer none -- -max_total_time=60
```

### vc-revocation-registry — `fuzz_credential_id`

Exercises `revoke` / `unrevoke` with arbitrary credential-ID byte payloads.
Asserts:

- IDs ≤ 256 bytes are always accepted.
- IDs > 256 bytes are always rejected.
- Revoke → unrevoke → revoke round-trips work correctly.

```sh
cd contracts/vc-revocation-registry
cargo +nightly fuzz run fuzz_credential_id --sanitizer none
```

Bounded run:

```sh
cargo +nightly fuzz run fuzz_credential_id --sanitizer none -- -max_total_time=60
```

---

## Why `--sanitizer none`?

Soroban contracts use `#![no_std]`. On many platforms AddressSanitizer (ASAN)
fails because `no_std` lacks the expected sanitizer initialization
infrastructure. Because contracts run in a WASM sandbox with no raw memory
access, memory-safety bugs are not the primary fuzzing target — **logic bugs
are**. Coverage-guided fuzzing without ASAN is fully effective for invariant
checking.

---

## Replaying a crash

Crash inputs are saved to `fuzz/artifacts/<target>/`. To replay a specific
crash:

```sh
# Example: replay a crash in fuzz_issuer_metadata
cd contracts/vc-issuer-registry
cargo +nightly fuzz run fuzz_issuer_metadata --sanitizer none \
    fuzz/artifacts/fuzz_issuer_metadata/<crash-file>
```

---

## Full campaign (manual / nightly)

Remove the `-max_total_time` flag to run indefinitely:

```sh
cd contracts/vc-issuer-registry
cargo +nightly fuzz run fuzz_issuer_metadata --sanitizer none
```

Corpus entries that improve coverage are written to
`fuzz/corpus/<target>/` (excluded from git via `.gitignore`).

---

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `error: the option Z is only accepted on the nightly compiler` | Running `cargo fuzz` without nightly | Add `+nightly` or `rustup override set nightly` in the contract directory |
| `Undefined symbols: ___sanitizer_cov_*` | ASAN + `no_std` on macOS aarch64 | Add `--sanitizer none` |
| `error: no such command: fuzz` | cargo-fuzz not installed | `cargo install cargo-fuzz` |
| `error[E0463]: can't find crate for 'std'` | Building fuzz target with wrong target triple | Ensure you run from the contract's directory, not the workspace root |
