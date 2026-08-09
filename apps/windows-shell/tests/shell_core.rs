use clip_ffi::{AppliedClipFfi, SessionError};
use std::sync::Arc;
use windows_shell::{
    ArmedStateStoring, ClipboardSyncController, ClipSession, FakeClipboard,
    InMemoryArmedStateStore, InMemoryCredentialStore, InMemoryNicknameStore, LinkKeyStoring,
    LocalClipboardSnapshot, LocalNicknameStoring, PasteboardEchoGuard, ShellCredentials,
};

struct RecordingSession {
    armed: std::sync::Mutex<bool>,
    next_applied: std::sync::Mutex<Option<AppliedClipFfi>>,
    published: std::sync::Mutex<Vec<String>>,
}

impl RecordingSession {
    fn new(armed: bool) -> Arc<Self> {
        Arc::new(Self {
            armed: std::sync::Mutex::new(armed),
            next_applied: std::sync::Mutex::new(None),
            published: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn published(&self) -> Vec<String> {
        self.published.lock().unwrap().clone()
    }
}

struct ArcSession(Arc<RecordingSession>);

impl ClipSession for ArcSession {
    fn is_armed(&self) -> bool {
        *self.0.armed.lock().unwrap()
    }

    fn is_sync_idle(&self) -> bool {
        false
    }

    fn set_armed(&self, armed: bool) {
        *self.0.armed.lock().unwrap() = armed;
    }

    fn publish_text(&self, text: String) -> Result<(), SessionError> {
        self.0.published.lock().unwrap().push(text);
        Ok(())
    }

    fn publish_text_and_image(
        &self,
        text: String,
        _image_bytes: Vec<u8>,
        _image_mime: String,
    ) -> Result<(), SessionError> {
        self.0.published.lock().unwrap().push(text);
        Ok(())
    }

    fn poll_applied(&self) -> Option<AppliedClipFfi> {
        self.0.next_applied.lock().unwrap().take()
    }
}

#[test]
fn echo_guard_ignores_remote_write_once() {
    let guard = PasteboardEchoGuard::new();
    guard.mark_remote_write("remote clip");
    assert!(guard.should_ignore_change(Some("remote clip")));
    assert!(!guard.should_ignore_change(Some("user copy")));
}

#[test]
fn armed_store_round_trip_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("armed.json");
    let mut store = windows_shell::ArmedStateStore::open(&path).unwrap();
    assert!(store.is_armed());
    store.set_armed(false);
    store.set_quit_opted_out(true);
    let reopened = windows_shell::ArmedStateStore::open(&path).unwrap();
    assert!(!reopened.is_armed());
    assert!(reopened.quit_opted_out());
}

#[test]
fn credentials_round_trip_memory_and_file() {
    let mut mem = InMemoryCredentialStore::default();
    let creds = ShellCredentials {
        ephemeral_id: vec![1; 16],
        link_key: vec![2; 32],
        relay_ws_url: clip_ffi::default_relay_ws_url(),
    };
    mem.save(&creds).unwrap();
    assert_eq!(mem.load().unwrap().as_ref(), Some(&creds));

    let dir = tempfile::tempdir().unwrap();
    let mut file = windows_shell::FileCredentialStore::new(dir.path().join("credentials.json"));
    file.save(&creds).unwrap();
    assert_eq!(file.load().unwrap().as_ref(), Some(&creds));
    file.delete().unwrap();
    assert!(file.load().unwrap().is_none());
}

#[test]
fn nickname_empty_clears() {
    let mut store = InMemoryNicknameStore::default();
    store.save("Desk PC");
    assert_eq!(store.load().as_deref(), Some("Desk PC"));
    store.save("   ");
    assert!(store.load().is_none());
}

#[test]
fn sync_publishes_local_text_when_armed() {
    let clipboard = FakeClipboard {
        change_count: 1,
        ..FakeClipboard::default()
    };
    let mut controller = ClipboardSyncController::new(clipboard);
    let session = RecordingSession::new(true);
    controller.attach(Box::new(ArcSession(Arc::clone(&session))));

    {
        let clip = controller.clipboard_mut();
        clip.change_count = 2;
        clip.snapshot = LocalClipboardSnapshot::text("hello from device");
    }
    controller.poll_local_clipboard();
    assert_eq!(session.published(), vec!["hello from device".to_string()]);
}

#[test]
fn sync_ignores_echo_after_remote_apply() {
    let clipboard = FakeClipboard {
        change_count: 1,
        ..FakeClipboard::default()
    };
    let mut controller = ClipboardSyncController::new(clipboard);
    let session = RecordingSession::new(true);
    session.next_applied.lock().unwrap().replace(AppliedClipFfi {
        created_at: 1,
        id_hex: "ab".into(),
        image_bytes: None,
        image_mime: None,
        text: "from remote device".into(),
    });
    controller.attach(Box::new(ArcSession(Arc::clone(&session))));

    controller.poll_remote_applied();
    assert_eq!(
        controller.clipboard_mut().written_texts,
        vec!["from remote device".to_string()]
    );

    {
        let clip = controller.clipboard_mut();
        clip.change_count += 1;
        clip.snapshot = LocalClipboardSnapshot::text("from remote device");
    }
    controller.poll_local_clipboard();
    assert!(session.published().is_empty());
}

#[test]
fn sync_does_not_publish_when_paused() {
    let clipboard = FakeClipboard {
        change_count: 1,
        ..FakeClipboard::default()
    };
    let mut controller = ClipboardSyncController::new(clipboard);
    let session = RecordingSession::new(false);
    controller.attach(Box::new(ArcSession(Arc::clone(&session))));
    {
        let clip = controller.clipboard_mut();
        clip.change_count = 3;
        clip.snapshot = LocalClipboardSnapshot::text("paused copy");
    }
    controller.poll_local_clipboard();
    assert!(session.published().is_empty());
}

#[test]
fn lifetime_may_auto_start_respects_quit_opt_out() {
    let snap = clip_ffi::LifetimeSnapshotFfi {
        durable_armed: true,
        elevated_capture_granted: true,
        has_link_key: true,
        quit_opted_out: false,
        requires_elevated_capture: false,
    };
    assert!(clip_ffi::lifetime_may_auto_start(snap.clone()));
    let opted = clip_ffi::LifetimeSnapshotFfi {
        quit_opted_out: true,
        ..snap
    };
    assert!(!clip_ffi::lifetime_may_auto_start(opted));
}

#[test]
fn in_memory_armed_defaults_armed() {
    let store = InMemoryArmedStateStore::default();
    assert!(store.is_armed());
    assert!(!store.quit_opted_out());
}
