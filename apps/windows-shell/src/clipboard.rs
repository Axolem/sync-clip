use clip_ffi::AppliedClipFfi;

/// Local clipboard snapshot captured by the Shell.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalClipboardSnapshot {
    pub image_bytes: Option<Vec<u8>>,
    pub image_mime: Option<String>,
    pub text: Option<String>,
}

impl LocalClipboardSnapshot {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            image_bytes: None,
            image_mime: None,
            text: Some(text.into()),
        }
    }

    pub fn is_empty(&self) -> bool {
        let text_empty = self.text.as_ref().map(|t| t.is_empty()).unwrap_or(true);
        let image_empty = self
            .image_bytes
            .as_ref()
            .map(|b| b.is_empty())
            .unwrap_or(true);
        text_empty && image_empty
    }
}

/// Platform pasteboard seam used by `ClipboardSyncController`.
pub trait SystemClipboard {
    /// Observe the OS clipboard and return a monotonic change counter.
    fn poll_change_count(&mut self) -> u64;
    fn read_snapshot(&mut self) -> LocalClipboardSnapshot;
    fn write_applied(&mut self, applied: &AppliedClipFfi);
}

/// In-memory clipboard for tests.
#[derive(Debug, Default)]
pub struct FakeClipboard {
    pub change_count: u64,
    pub snapshot: LocalClipboardSnapshot,
    pub written_texts: Vec<String>,
}

impl SystemClipboard for FakeClipboard {
    fn poll_change_count(&mut self) -> u64 {
        self.change_count
    }

    fn read_snapshot(&mut self) -> LocalClipboardSnapshot {
        self.snapshot.clone()
    }

    fn write_applied(&mut self, applied: &AppliedClipFfi) {
        self.written_texts.push(applied.text.clone());
        self.snapshot = LocalClipboardSnapshot::text(applied.text.clone());
        self.change_count = self.change_count.saturating_add(1);
    }
}
