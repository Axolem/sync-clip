# Rust Clip Engine behind native Shells

Sync behavior, Clip modeling, and end-to-end crypto must be identical on every platform. The Clip Engine is implemented in Rust and linked into each Shell over FFI. macOS and Android Shells stay fully native for clipboard APIs, background lifetime, Link Key storage, and UI. Kotlin Multiplatform and a per-machine TypeScript daemon were rejected to avoid duplicated policy logic or an extra always-on runtime.
