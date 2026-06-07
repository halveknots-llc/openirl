# Progress

**Phase:** ITER-0000 validation baseline
**Task:** Restore trustworthy local validation before product-scope work
**Sentinel corpus:** initial Rust and repository validation commands
**Last event:** Baseline found AppleDouble sidecars, an SVG raw-string parse failure, rustfmt drift, and a Clippy lint-priority failure.

## Current Implementation Plan

1. Clean non-product AppleDouble sidecars so repository validators read real source files.
2. Repair Rust parser and lint blockers without weakening validation.
3. Run the requested static, audit, format, Clippy, test, and xtask commands.
4. Use paired reviewer findings to choose the next smallest product slice with observable evidence.
5. Add or update project operating instructions after the validation baseline is reliable.
