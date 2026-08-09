use clip_ffi::{AppliedClipFfi, Session, SessionError};
use std::sync::Arc;

/// Session seam for Shell sync loops (`clip_ffi::Session` + test doubles).
pub trait ClipSession: Send {
    fn is_armed(&self) -> bool;
    fn is_sync_idle(&self) -> bool;
    fn poll_applied(&self) -> Option<AppliedClipFfi>;
    fn publish_text(&self, text: String) -> Result<(), SessionError>;
    fn publish_text_and_image(
        &self,
        text: String,
        image_bytes: Vec<u8>,
        image_mime: String,
    ) -> Result<(), SessionError>;
    fn set_armed(&self, armed: bool);
}

impl ClipSession for Arc<Session> {
    fn is_armed(&self) -> bool {
        Session::is_armed(self)
    }

    fn is_sync_idle(&self) -> bool {
        Session::is_sync_idle(self)
    }

    fn set_armed(&self, armed: bool) {
        Session::set_armed(self, armed);
    }

    fn publish_text(&self, text: String) -> Result<(), SessionError> {
        Session::publish_text(self, text)
    }

    fn publish_text_and_image(
        &self,
        text: String,
        image_bytes: Vec<u8>,
        image_mime: String,
    ) -> Result<(), SessionError> {
        Session::publish_text_and_image(self, text, image_bytes, image_mime)
    }

    fn poll_applied(&self) -> Option<AppliedClipFfi> {
        Session::poll_applied(self)
    }
}
