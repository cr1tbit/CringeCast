# CringeCast Rust Rewrite Prep

This directory is a preparation scaffold for reimplementing CringeCast in Rust with a clean split:

- `core` domain logic that is hardware-agnostic
- `platform` adapters for Linux shell scripts now and ESP32 later

## Why this structure

Current Python app mixes these concerns in one place:

- HTTP routing and auth checks
- anti-raid `teapot` state
- text sanitation and sentence splitting
- language detection and TTS invocation
- direct shell execution for audio/volume controls

The Rust prep isolates the reusable domain logic so transport (HTTP/MQTT/serial) and hardware backend can be swapped independently.

## Files

- `src/core.rs`
  - `CringeService` orchestrator with app behavior
  - `AudioBackend` trait for hardware side effects
  - `LanguageDetector` trait for auto language selection
  - `TeapotMode` state machine and anti-raid timing logic
  - `sanitize` and `smart_split` helpers
- `src/platform/linux_shell.rs`
  - Linux adapter executing existing shell scripts (`speak.sh`, `play.sh`, etc.)
- `src/platform/mock.rs`
  - In-memory backend for tests and local dev without audio hardware

## Endpoint Compatibility Plan

These Python endpoints should map directly onto Rust handlers later:

- `GET /` -> serve frontend
- `GET /say/:saying` -> `service.speak(saying, Some("en"))`
- `GET /mow/:saying` -> `service.speak(saying, Some("pl"))`
- `GET /guess/:saying` and `GET /:saying` -> `service.speak(saying, None)`
- `GET /play/:category/:filename` -> `service.play_file(...)`
- `GET /stop` -> `service.stop()`
- `GET /vol` -> `service.get_volume()`
- `GET /vol/:value` -> `service.set_volume(value)`
- `GET /getFilelist` -> `service.list_files()`
- `GET /teapot/:target_state` -> `service.teapot_control(...)`

## Migration Sequence

1. Add Rust HTTP crate (`axum` recommended) that wires routes to `CringeService`.
2. Keep existing static frontend and API path contract unchanged.
3. Replace Python service with Rust binary on NanoPi while still using shell scripts.
4. Replace shell scripts with native Rust audio + mixer calls on Linux (optional).
5. Add `esp32` backend implementing `AudioBackend` and reusing the same `core` crate.

## ESP32 Readiness Notes

For ESP32, avoid Linux assumptions in `core`:

- no shelling out in domain code
- no filesystem assumptions in domain code
- no direct HTTP framework types in domain code

Implement a separate backend for ESP32 that maps `speak`, `play_file`, and volume APIs to board-specific drivers.

## Gaps intentionally left for implementation phase

- No HTTP server crate included yet
- No upload endpoint handling yet
- No production language detector yet (currently trait-based placeholder)

