# Clip wire envelope and relay API

- **Status:** accepted (issue #2 approved 2026-08-08)
- **Date:** 2026-08-08
- **Extends:** [ADR-0001](0001-encrypted-relay.md), [ADR-0002](0002-latest-only-buffer.md), [ADR-0003](0003-rust-clip-engine.md), [ADR-0004](0004-plain-text-clips.md)
- **Detail:** [docs/protocol/clip-wire-v0.md](../protocol/clip-wire-v0.md)

## Context

Before AFK implementation of the Clip Engine and relay, humans must approve how Sync Group channels are addressed from Link Key material, how ciphertext envelopes look on the wire, and how the relay enforces latest-only buffering with a short TTL.

ADR-0001 chose an encrypted relay that never sees plaintext. ADR-0002 chose latest-only-per-Sync-Group buffering with ~15 minute TTL. ADR-0003 places sync policy and crypto in the Rust Clip Engine. ADR-0004 constrains Clip text to plain text with optional image parts. This ADR records the concrete wire sketch those decisions imply.

## Decision

1. **Logical Clip model (pre-encryption)** is defined in the Clip Engine: `id`, `created_at`, plain text part, optional image part (`mime` + bytes), and a sender Device ephemeral id used only for echo suppression (not user identity; random per install is fine). Local Nickname is never included.
2. **Link Key → channel + keys** uses a single HKDF-SHA256 scheme with fixed labels so implementers do not invent a second derivation. Public **channel id** and AEAD keys are derived; the Link Key itself is never sent to the relay.
3. **Wire envelope** is ciphertext-only to the relay: channel id + AEAD ciphertext + minimal routing metadata (nonce, version). AEAD algorithm: **XChaCha20-Poly1305**. Plaintext of the AEAD is a versioned Clip serialization (see protocol note).
4. **Relay API** exposes publish and subscribe of ciphertext for a channel. Buffering is **latest-only per Sync Group channel** with **~15 minute TTL**; a newer Clip supersedes any buffered envelope. Relay never decrypts.
5. **Last-write-wins (LWW)** and **echo suppression** are Clip Engine responsibilities on each Device. Shells do not re-broadcast Clips that arrived remotely and were applied to the local clipboard.
6. **Armed / Paused** are Engine-level: the relay may still deliver while Paused; the Clip Engine drops inbound application and skips outbound publish until Armed again.

## Consequences

- Follow-on Engine and relay work can cite this ADR + `docs/protocol/clip-wire-v0.md` without re-opening transport or crypto shape.
- Changing AEAD, KDF labels, or buffer semantics after approval requires a new ADR (or explicit supersession of this one).
- Out of scope for v0 wire: rich text, multi Sync Group per Device, per-Device revoke, P2P / LAN discovery.

## Locked defaults (approval)

- AEAD + KDF: **XChaCha20-Poly1305** + **HKDF-SHA256** with labels `sync-clip/v0/channel-id` and `sync-clip/v0/clip-aead-key`
- Clip plaintext encoding: **CBOR** (deterministic / sorted keys)
- Channel id on the wire: **lowercase hex** (32 chars)
- Buffer TTL: **~15 minutes**, resets on every superseding publish
- Relay auth (v0): channel id + rate limits only (no derived relay token)
- First-party transport: **WebSocket-first**
- Soft ciphertext cap: **2 MiB**

## Approval

Accepted via issue #2. Algorithms and field layouts in [clip-wire-v0.md](../protocol/clip-wire-v0.md) are the v0 production protocol for follow-on Engine/relay work.
