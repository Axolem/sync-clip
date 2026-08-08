# Clip wire protocol v0

**Status:** accepted (GitHub issue #2 approved 2026-08-08)  
**Normative decision record:** [ADR-0005](../adr/0005-clip-wire-and-relay-api.md)  
**Depends on:** [ADR-0001](../adr/0001-encrypted-relay.md) (encrypted relay), [ADR-0002](../adr/0002-latest-only-buffer.md) (latest-only + TTL), [ADR-0003](../adr/0003-rust-clip-engine.md) (Clip Engine owns policy/crypto), [ADR-0004](../adr/0004-plain-text-clips.md) (plain text + optional images)

This note is the v0 implementer protocol. Field layouts and algorithms below are frozen for AFK Engine/relay work unless superseded by a new ADR.

Domain terms follow root `CONTEXT.md`: Clip, Device, Link Key, Sync Group, Clip Engine, Shell, Armed, Paused, Local Nickname.

---

## 1. Clip logical model (before encryption)

A Clip is a clipboard snapshot owned by the Clip Engine. Serialization below is the **plaintext** input to AEAD — never visible to the relay.

| Field | Type | Purpose |
| --- | --- | --- |
| `schema_version` | u8 | Clip plaintext schema; v0 = `1` |
| `id` | 16 bytes | Opaque Clip id (random UUID v4 bytes recommended) |
| `created_at` | i64 | Unix time in **milliseconds** when the Shell captured the Clip (sender clock). Used with `id` for LWW tie-breaks |
| `text` | UTF-8 string | Plain text part; empty string if image-only. Rich text is already normalized away per ADR-0004 |
| `image` | optional | Present only when an image part is included |
| `image.mime` | string | e.g. `image/png`, `image/jpeg` |
| `image.bytes` | bytes | Raw image payload (oversized images omitted on capture; text may still sync) |
| `sender_ephemeral_id` | 16 bytes | Random per Device install. Used **only** for echo suppression. Not user identity; never a Local Nickname; not stable across reinstall unless Shell persists it |

**Explicitly excluded from Clip plaintext:** Local Nickname, accounts, OS usernames, durable member lists.

### Suggested plaintext encoding (v0)

Canonical encoding for AEAD input (implementations MUST agree):

1. Encode as **CBOR** map with **sorted keys** (or equivalent deterministic CBOR), keys as text strings:
   - `"created_at"` → integer (ms)
   - `"id"` → bstr (16)
   - `"image"` → map `{"bytes": bstr, "mime": tstr}` if present; omit key if absent
   - `"schema_version"` → integer `1`
   - `"sender_ephemeral_id"` → bstr (16)
   - `"text"` → tstr (may be `""`)
**Accepted encoding:** CBOR as above (deterministic / sorted keys).

---

## 2. Link Key → channel and encryption material

### 2.1 Link Key representation

- Link Key is a shared secret admitting a Device into a Sync Group.
- Proposed generation: **32 random bytes**, encoded for humans as **base32** (no padding) when displayed/entered. Exact UX encoding is Shell concern; Clip Engine should accept raw 32 bytes after decode.
- Rotating the Link Key replaces the Sync Group credential for every Device (no per-Device revoke) — ADR-0001 / CONTEXT.

### 2.2 KDF (single scheme)

**Algorithm:** HKDF-SHA256  
**IKM:** Link Key bytes (32)  
**Salt:** empty (fixed; Link Key entropy is the salt)  
**Info / labels:** ASCII strings below (exact bytes; no trailing NUL)

| Output | Length | HKDF `info` label | Visibility |
| --- | --- | --- | --- |
| Channel id | 16 bytes | `sync-clip/v0/channel-id` | **Public** — used in relay URLs / subscribe filters |
| Clip AEAD key | 32 bytes | `sync-clip/v0/clip-aead-key` | Private — Clip Engine only |

Derivation for each output independently:

```
OKM = HKDF-SHA256(ikm = LinkKey, salt = "", info = <label>, L = <length>)
```

**Channel id on the wire (accepted):** lowercase hex of the 16 bytes (32 hex chars).

There is **one** derivation scheme. Do not invent per-platform KDFs, separate “pairing salts,” or relay-issued channel names for v0.

### 2.3 AEAD

| Item | Choice (accepted) |
| --- | --- |
| Algorithm | **XChaCha20-Poly1305** (IETF / libsodium-compatible construction) |
| Key | 32-byte `clip-aead-key` from HKDF |
| Nonce | 24 random bytes per publish (never reuse with same key) |
| AAD | `channel_id` bytes (raw 16-byte channel id), so ciphertext is bound to the Sync Group channel |
| Plaintext | CBOR Clip from §1 |

**Library sketch (non-normative):** Rust `chacha20poly1305` (`XChaCha20Poly1305`), or libsodium `crypto_aead_xchacha20poly1305_ietf_*`. Prefer audited libs over hand-rolled AEAD.

### 2.4 Wire envelope (what the relay sees)

The relay stores/forwards **only**:

| Field | Type | Notes |
| --- | --- | --- |
| `protocol_version` | u8 | Wire envelope version; v0 = `1` |
| `channel_id` | 16 bytes / hex | Public Sync Group address |
| `nonce` | 24 bytes | AEAD nonce |
| `ciphertext` | bytes | XChaCha20-Poly1305 ciphertext \|\| tag |
| `published_at` | i64 ms | Set by **relay** on accept (server clock) for TTL; optional client hint ignored for TTL authority |

No plaintext Clip fields. No Local Nickname. No Device user identity.

---

## 3. Relay API sketch

Transport may be **WebSocket** (preferred for live push) or **HTTP publish + SSE subscribe**. Semantics are identical.

### 3.1 Concepts

- One **channel** ≡ one Sync Group address (`channel_id`).
- Buffer: **at most one** envelope per channel (latest-only) — ADR-0002.
- TTL: **~15 minutes** from relay `published_at`; expired buffer is deleted (empty channel).
- Newer Clip **supersedes** any buffered envelope for that channel (replace, do not queue).

### 3.2 WebSocket sketch

Connect: `wss://<relay>/v0/ws`

**Client → relay**

```json
{ "type": "subscribe", "channel_id": "<32-hex>" }
{ "type": "unsubscribe", "channel_id": "<32-hex>" }
{ "type": "publish", "channel_id": "<32-hex>", "protocol_version": 1, "nonce": "<base64>", "ciphertext": "<base64>" }
```

**Relay → client**

```json
{ "type": "envelope", "channel_id": "<32-hex>", "protocol_version": 1, "nonce": "<base64>", "ciphertext": "<base64>", "published_at": 1710000000000 }
{ "type": "empty", "channel_id": "<32-hex>" }
{ "type": "error", "code": "bad_request|unauthorized|rate_limited", "message": "..." }
```

On successful `subscribe`, relay immediately sends the current buffered envelope if present and unexpired; otherwise `empty`.

On `publish`, relay:

1. Validates size limits (suggested soft caps: ciphertext ≤ 2 MiB; reject larger).
2. Replaces any existing buffer for `channel_id`.
3. Sets `published_at` to server now; starts/resets ~15 min TTL.
4. Fans out `envelope` to all current subscribers on that channel (including publisher — Shell/Engine may ignore via echo rules).

### 3.3 HTTP + SSE equivalent

- `POST /v0/channels/{channel_id}/envelopes` — body: `{ protocol_version, nonce, ciphertext }` → 202; same replace + TTL rules.
- `GET /v0/channels/{channel_id}/stream` (SSE) — events: `envelope`, `empty`; initial event = current buffer or empty.

Auth for v0: **knowledge of `channel_id` is not authentication of the Link Key** (channel id is public-ish derived material). Optional hardening (out of band for this issue): rate limits, CAPTCHA on abusive publish, or a second HKDF “relay token” if human review demands it. Default proposal: channel id alone + rate limits; ciphertext privacy still depends on Link Key.

### 3.4 Latest-only + TTL (normative behavior)

| Event | Buffer state |
| --- | --- |
| Publish A | Buffer = A; TTL clock starts |
| Publish B (any time before A expires) | Buffer = B only; A discarded; TTL resets |
| TTL elapses with no newer publish | Buffer cleared |
| Subscribe while buffer live | Deliver current envelope once, then live updates |
| Subscribe after TTL | `empty`; wait for future publish |

No durable history. No multi-Clip queue. Matches ADR-0002.

---

## 4. LWW and echo suppression (Clip Engine)

### 4.1 Last-write-wins

When the Clip Engine decrypts a remote envelope successfully:

1. Compare remote Clip to the Engine’s **last applied Clip** for this Sync Group (if any).
2. Remote wins if `created_at` is greater; if equal, remote wins if `id` is lexicographically greater (bytewise).
3. If remote loses, discard (do not write OS clipboard; do not treat as new local origin).
4. If remote wins, apply to Shell clipboard path and record as last applied.

Sender clock skew can cause rare inversions; acceptable for v0 clipboard sync. Do not use relay `published_at` for LWW of Clip content (that clock is for TTL only).

### 4.2 Echo suppression

Copying a remote Clip into the OS clipboard must not republish forever.

Proposed Engine rules:

1. Remember `sender_ephemeral_id` + `id` (and optionally a short hash of text/image) of the last **remotely applied** Clip.
2. When the Shell reports a local clipboard change while Armed:
   - If the change matches the last remotely applied Clip (same content and/or same id tracked via Shell write marker), **do not publish**.
   - Else build a new Clip with this Device’s `sender_ephemeral_id` and publish.
3. Optionally ignore inbound Clips whose `sender_ephemeral_id` equals this Device’s own id (self-echo from relay fanout). Still apply LWW if another Device somehow reused ids (should not happen).

Local Nickname is never part of echo logic.

---

## 5. Armed and Paused

**Prefer Engine-level** (ADR-0003: policy lives in Clip Engine).

| State | Outbound publish | Inbound apply to clipboard |
| --- | --- | --- |
| **Armed** | Yes (on local clipboard changes, after echo checks) | Yes (after decrypt + LWW) |
| **Paused** | No | No — Engine drops application even if relay delivers |

While Paused:

- Shell may keep the WebSocket/SSE open (reconnect simplicity) or disconnect; either is fine.
- Relay may still buffer/deliver envelopes for the channel.
- Clip Engine **must not** ask the Shell to write the clipboard and **must not** publish.
- Device remains in the Sync Group (same Link Key); Paused is not leave/logout.

Armed/Paused are **not** relay protocol messages in v0.

---

## 6. Size and capture notes (Shell → Engine)

Aligned with CONTEXT / ADR-0004:

- Text: plain only; empty allowed if image present.
- Images: omit when oversized (Shell/Engine policy; suggest ≤ ~1–1.5 MiB encoded before encrypt); text still syncs when present.
- One Sync Group per Device in v1.

---

## 7. Out of scope (v0)

- Rich text / HTML / RTF round-trip
- Multiple Sync Groups per Device
- Per-Device revoke without Link Key rotation
- P2P, LAN-only discovery, or relay-free modes
- Durable Clip history on relay or Device beyond “last applied” for LWW/echo
- User identity, accounts, Local Nickname on the wire

---

## 8. Locked defaults (issue #2 approval)

1. **XChaCha20-Poly1305 + HKDF-SHA256** with labels in §2.2 — accepted
2. **CBOR** plaintext encoding — accepted
3. Channel id encoding: **lowercase hex** — accepted
4. **~15 minute** TTL; resets on every superseding publish — accepted
5. Relay auth: channel id + rate limits only — accepted
6. **WebSocket-first** for the first-party relay — accepted
7. Soft ciphertext cap **2 MiB** — accepted

---

## 9. Approval checklist (issue #2)

- [x] Clip envelope fields specified (text, optional image, timestamps/ids for LWW, echo metadata)
- [x] Relay API specified for publish/subscribe of ciphertext only
- [x] Latest-only-per-Sync-Group + ~15 minute TTL specified
- [x] Link Key → channel/session material derivation specified as one scheme
- [x] Decision recorded in-repo (this note + ADR-0005) for follow-on issues
