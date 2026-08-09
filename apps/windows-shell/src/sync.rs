use crate::clipboard::{LocalClipboardSnapshot, SystemClipboard};
use crate::echo::PasteboardEchoGuard;
use crate::session::ClipSession;
use std::sync::{Arc, Mutex};

/// Coordinates clipboard watch + Session publish/poll for the Windows Shell.
pub struct ClipboardSyncController<C: SystemClipboard> {
    clipboard: C,
    echo: PasteboardEchoGuard,
    last_change_count: u64,
    session: Option<Arc<Mutex<Box<dyn ClipSession>>>>,
}

impl<C: SystemClipboard> ClipboardSyncController<C> {
    pub fn new(clipboard: C) -> Self {
        Self {
            clipboard,
            echo: PasteboardEchoGuard::new(),
            last_change_count: 0,
            session: None,
        }
    }

    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    pub fn is_armed(&self) -> bool {
        self.session
            .as_ref()
            .and_then(|s| s.lock().ok())
            .map(|s| s.is_armed())
            .unwrap_or(false)
    }

    pub fn is_sync_idle(&self) -> bool {
        self.session
            .as_ref()
            .and_then(|s| s.lock().ok())
            .map(|s| s.is_sync_idle())
            .unwrap_or(false)
    }

    pub fn attach(&mut self, session: Box<dyn ClipSession>) {
        self.last_change_count = self.clipboard.poll_change_count();
        self.session = Some(Arc::new(Mutex::new(session)));
    }

    pub fn detach(&mut self) {
        self.session = None;
    }

    /// Mutable clipboard access for Shell loops and tests.
    pub fn clipboard_mut(&mut self) -> &mut C {
        &mut self.clipboard
    }

    pub fn set_armed(&self, armed: bool) {
        if let Some(session) = &self.session {
            if let Ok(s) = session.lock() {
                s.set_armed(armed);
            }
        }
    }

    /// Single-step local watch (tests + poll loop).
    pub fn poll_local_clipboard(&mut self) {
        let change_count = self.clipboard.poll_change_count();
        let Some(session) = &self.session else {
            return;
        };
        let Ok(session) = session.lock() else {
            return;
        };
        if !session.is_armed() {
            return;
        }
        if change_count == self.last_change_count {
            return;
        }
        self.last_change_count = change_count;
        let snapshot = self.clipboard.read_snapshot();
        if self.echo.should_ignore_change(snapshot.text.as_deref()) {
            return;
        }
        if snapshot.is_empty() {
            return;
        }
        publish(session.as_ref(), &snapshot);
    }

    /// Single-step remote poll (tests + poll loop).
    pub fn poll_remote_applied(&mut self) {
        let Some(session) = &self.session else {
            return;
        };
        let Ok(session) = session.lock() else {
            return;
        };
        if !session.is_armed() {
            return;
        }
        let Some(applied) = session.poll_applied() else {
            return;
        };
        self.echo.mark_remote_write(&applied.text);
        self.clipboard.write_applied(&applied);
        self.last_change_count = self.clipboard.poll_change_count();
    }
}

fn publish(session: &dyn ClipSession, snapshot: &LocalClipboardSnapshot) {
    let text = snapshot.text.clone().unwrap_or_default();
    let result = if let (Some(bytes), Some(mime)) =
        (snapshot.image_bytes.as_ref(), snapshot.image_mime.as_ref())
    {
        session.publish_text_and_image(text, bytes.clone(), mime.clone())
    } else if !text.is_empty() {
        session.publish_text(text)
    } else {
        Ok(())
    };
    if let Err(err) = result {
        eprintln!("sync-clip: publish failed (soft): {err}");
    }
}
