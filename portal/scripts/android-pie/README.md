# Android (Termux) static-PIE build for heart-portal

## Root cause this works around

Android 10+ refuses non-PIE executables (e_type must be ET_DYN), and bionic rejects
TLS segments with effective alignment < 64. A fully static binary (musl, no PT_INTERP)
satisfies the kernel, but `rustc`'s `aarch64-unknown-linux-musl` target does NOT
support static PIE (`static_position_independent_executables = false`): with
`-C target-feature=+crt-static -C relocation-model=pic -C link-arg=-pie` it still
links the plain-static startup object `crt1.o`. The resulting binary has ~24k
`.rela.dyn` relocations that are never applied at startup -> first indirect jump
lands on garbage -> SIGSEGV.

Fix: swap `crt1.o` for `rcrt1.o` (musl's self-relocating startup object) at link
time. This directory automates that swap.

## How it works

- `build-android-pie.ps1` — sets CC/AR to cargo-zigbuild's aarch64-musl wrappers,
  then runs `cargo rustc` with `-C linker=loglld.bat`.
- `loglld.bat` — rustc calls it with an `@response-file`. It forwards the original
  call to `rust-lld -flavor gnu` (rustc does not pass `-flavor` to unknown .bat
  linkers) and, while rustc's temporary object files still exist, also runs
  `rcrtlink.ps1`.
- `rcrtlink.ps1` — copies the response file, replaces `self-contained\crt1.o`
  with `self-contained\rcrt1.o`, redirects `-o` to `heart-portal-pie`, and links
  the fixed binary with rust-lld.

Note: the swap must happen inside the linker wrapper. rustc deletes its cgu
object files right after linking, so a post-hoc re-link cannot work.

## Build

From `portal/` (or repo root — script cds itself):

```powershell
powershell -File scripts/android-pie/build-android-pie.ps1
```

Output: `target/aarch64-unknown-linux-musl/release/heart-portal-pie`
(unstripped, ~17 MB; symbols kept for postmortem).

Source-side prerequisites already in this branch:
- `#[repr(align(64))]` TLS pad in `main.rs` (forces PT_TLS align 64, skew 0).
- `unix_service.rs` (stop/status via /proc scan on non-Windows/macOS).

## Verify before shipping (three layers)

1. ELF static checks (e_type=3, PT_TLS align 64 skew 0, no PT_INTERP).
2. Differential startup check: `_start_c` must be the 488-byte self-relocation
   routine (contains the relocation write-back loop), not the 40-byte direct
   jump to `__libc_start_main`.
3. Functional smoke on real Linux: `qemu-aarch64-static ./heart-portal-pie status`
   must exit 0 (e.g. on a GitHub Actions ubuntu runner — see
   https://github.com/HiddenFox/codex-async-kit/tree/hp-android-smoke for a
   self-reporting workflow example). qemu-user cannot run on Windows hosts;
   "PIE accepted by Android's loader" can only be confirmed on a real device.
