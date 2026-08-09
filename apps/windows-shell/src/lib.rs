//! Windows Shell core: stores, clipboard seam, and Session sync loops.
//!
//! The tray UI lives behind `cfg(windows)` in `main` / `tray`.

mod armed;
mod clipboard;
mod credentials;
mod echo;
mod nickname;
mod session;
mod sync;

pub use armed::{ArmedStateStore, ArmedStateStoring, InMemoryArmedStateStore};
pub use clipboard::{FakeClipboard, LocalClipboardSnapshot, SystemClipboard};
pub use credentials::{
    FileCredentialStore, InMemoryCredentialStore, LinkKeyStoring, ShellCredentials,
};
pub use echo::PasteboardEchoGuard;
pub use nickname::{InMemoryNicknameStore, LocalNicknameStoring, NicknameStore};
pub use session::ClipSession;
pub use sync::ClipboardSyncController;

/// Library version aligned with workspace package version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
