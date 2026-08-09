# Sync Clip

Cross-device clipboard sync: Devices in a Sync Group exchange Clips so a copy on one Device can be pasted on another with normal OS paste.

## Language

**Clip**:
A single clipboard snapshot captured at copy time on a Device. In v1 a Clip carries plain text and/or images together; rich text is normalized to plain text on capture. Oversized images are omitted; text still syncs when present.
_Avoid_: Message, payload, copy event, item, rich text document

**Device**:
One native install of the app on a single machine or phone that can join a Sync Group.
_Avoid_: Client, node, peer, user

**Link Key**:
The shared secret that admits a Device into a Sync Group. It is the only joining credential and carries no user identity. Rotating the Link Key replaces the Sync Group credential for every Device; there is no per-Device revoke.
_Avoid_: Account, password, login, pairing code, user id

**Sync Group**:
The set of Devices that share the same Link Key and therefore exchange Clips with each other. In v1 a Device belongs to at most one Sync Group at a time.
_Avoid_: Room, account, workspace, network

**Clip Engine**:
The shared, platform-agnostic core that owns the Clip model, end-to-end encryption, sync protocol, Armed/Paused rules, and echo suppression.
_Avoid_: Copy engine, core library, SDK, backend

**Shell**:
The platform-native app that owns OS clipboard read/write, Shell Lifetime, Link Key storage, and UI, and delegates sync behavior to the Clip Engine.
_Avoid_: Client wrapper, host app, frontend

**Shell Lifetime**:
Whether the Shell remains running on the Device so it can own clipboard access and Sync Group membership. Distinct from Armed and Paused.
_Avoid_: Listening, always-on, background sync, connected, online

**Armed**:
Device state where local clipboard changes are published as Clips and remote Clips are written to the system clipboard. Armed does not mean the Shell is running, and stopping the Shell is not the same as Pause.
_Avoid_: Online, connected, logged in, syncing

**Paused**:
Device state where the Device stays in its Sync Group but neither publishes nor accepts Clips. Paused does not require ending Shell Lifetime or leaving the Sync Group.
_Avoid_: Offline, disconnected, logged out, disabled, quit

**Elevated Clipboard Capture**:
On Android, the extra OS permission the Shell needs to observe local clipboard changes while another app is focused. Without it a Device cannot stay Armed; macOS menu bar Shells do not require this. iOS / iPadOS / visionOS Shells use foreground UIPasteboard access instead (no Elevated Clipboard Capture equivalent).
_Avoid_: Accessibility service (implementation), background clipboard hack, focus-only capture

**Sync Idle**:
Armed with Shell Lifetime up, but Clips are not flowing because the relay session is unavailable. The Device stays Armed and keeps retrying; Sync Idle is not Paused.
_Avoid_: Paused, disconnected, offline, soft-fail

**Local Nickname**:
An optional label stored only on a Device for its own UI. It is never sent to the Sync Group or relay as identity.
_Avoid_: Display name, username, device id, member name
