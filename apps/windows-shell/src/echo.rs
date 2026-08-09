use std::sync::Mutex;

/// Suppresses echo when the Shell writes a remote Clip to the system clipboard.
#[derive(Debug, Default)]
pub struct PasteboardEchoGuard {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    ignore_remaining: u32,
    last_applied_text: Option<String>,
}

impl PasteboardEchoGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_remote_write(&self, text: &str) {
        let mut g = self.inner.lock().expect("echo lock");
        g.ignore_remaining = g.ignore_remaining.saturating_add(1);
        g.last_applied_text = Some(text.to_string());
    }

    pub fn should_ignore_change(&self, current_text: Option<&str>) -> bool {
        let mut g = self.inner.lock().expect("echo lock");
        if g.ignore_remaining > 0 {
            g.ignore_remaining -= 1;
            return true;
        }
        if let (Some(current), Some(last)) = (current_text, g.last_applied_text.as_deref()) {
            if current == last {
                return true;
            }
        }
        false
    }

    pub fn reset(&self) {
        let mut g = self.inner.lock().expect("echo lock");
        g.ignore_remaining = 0;
        g.last_applied_text = None;
    }
}
