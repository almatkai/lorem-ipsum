# Lorem Ipsum ? Rust / Tezbar beta9

Standalone native process extension: protocol 1, manifest schema 3, no JS/Bun runtime.
Replaces `raycast.lorem-ipsum` and `com.tezbar.lorem-ipsum`; all six command IDs,
argument names and `action` preference values are preserved.

- `paragraphs`, `sentences`, `words`: optional count, default 1, maximum 1000.
- `markdown`, `html`: structured placeholder documents with headings, emphasis,
  paragraphs, lists and quotations, generated locally without AI.
- `ai-generate`: optional topic; typed `host.ai.ask` uses the configured Tezbar AI
  provider with the original prompt and model/creativity options.
- Output: clipboard, paste, or paste then copy, with the original success messages.

## Local checks and development package

```powershell
$env:CARGO_BUILD_JOBS='1'
cargo fmt --all -- --check
cargo test --locked
cargo build --locked
python scripts/vendor-licenses.py --check
python scripts/package.py --target x86_64-pc-windows-msvc --binary target/debug/tezbar-lorem-ipsum.exe --output dist/development
```

SDK revision is immutable in Cargo.toml and Cargo.lock. The package script checks
identity, command coverage, licenses, every packaged file hash and ZIP inventory.
Development ZIPs are not published updates. Pass `--profile release` when packaging
an optimized binary so the artifact sidecar records the actual build profile.

## Upstream parity and limits

Audited GitHub source: almatkai/lorem-ipsum commit
`921155fc202818ed8bc0356f136bb06005d80c12`; Raycast baseline
`6734d85992290066fd31a7f5bee3cca3d3545a1a` (extensions/lorem-ipsum).
Generator algorithm and 62-word dictionary match npm lorem-ipsum 2.0.10.
Zero words/sentences request random counts; zero paragraphs produce empty text.
Whitespace topics are retained, parseInt-style prefixes accepted, paragraphs use
CRLF blank lines. Original sources and fetched comparison evidence remain available.

Intentional robustness changes: negative counts return a validation error instead
of upstream RangeError; blank/non-text AI output and output over the host 1 MiB
limit return errors. Clipboard is written before hiding the window so failures
remain visible. Animated Raycast AI toasts are not available in this no-view
protocol result; completion/error is returned to the host. AI provider errors
retain useful host diagnostics. Paste clipboard restoration follows the host.

Windows unit/real-pipe protocol checks and the production offline importer are
covered; live GPUI, clipboard/focus and configured AI provider acceptance remain
pending independent verification. Native macOS and Windows unit/process suites each
passed 26 tests for version 0.1.1; optimized packages were built locally on both
platforms. macOS stripping was disabled because the toolchain's rust-objcopy could
not load libLLVM; no application signing policy was changed. The two structured
commands follow Raycast commit `5bcd41ff46cf8ad355e98f11bb7e716c6fdafc4b`;
the original word engine remains the documented 2.0.10-derived implementation.
App-wide pnpm checks belong to the parent integration; this standalone Rust change
does not modify the shared SDK, app, catalog, pins or STATUS.

## Licenses

Original author AntonNiklasson; contributors GastroGeek, stelo, enkia, j3lte.
Tezbar adaptation almatkai. See LICENSE, NOTICE and licenses/THIRD_PARTY.md.
