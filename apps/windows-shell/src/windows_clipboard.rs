//! Windows system clipboard via `arboard`.

use arboard::{Clipboard, ImageData};
use clip_ffi::AppliedClipFfi;
use image::ImageEncoder;
use windows_shell::{LocalClipboardSnapshot, SystemClipboard};

pub struct WindowsClipboard {
    change_count: u64,
    last_fingerprint: String,
}

impl WindowsClipboard {
    pub fn open() -> Result<Self, String> {
        let mut this = Self {
            change_count: 0,
            last_fingerprint: String::new(),
        };
        let _ = this.poll_change_count();
        Ok(this)
    }

    fn fingerprint(snapshot: &LocalClipboardSnapshot) -> String {
        let text = snapshot.text.clone().unwrap_or_default();
        let img_len = snapshot
            .image_bytes
            .as_ref()
            .map(|b| b.len())
            .unwrap_or(0);
        format!("{text}|{img_len}")
    }

    fn read_snapshot_inner(&self) -> LocalClipboardSnapshot {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(_) => return LocalClipboardSnapshot::default(),
        };
        let text = clipboard.get_text().ok();
        if let Ok(image) = clipboard.get_image() {
            if let Some(png) = rgba_to_png(&image) {
                return LocalClipboardSnapshot {
                    image_bytes: Some(png),
                    image_mime: Some("image/png".into()),
                    text,
                };
            }
        }
        LocalClipboardSnapshot {
            image_bytes: None,
            image_mime: None,
            text,
        }
    }
}

impl SystemClipboard for WindowsClipboard {
    fn poll_change_count(&mut self) -> u64 {
        let snapshot = self.read_snapshot_inner();
        let fp = Self::fingerprint(&snapshot);
        if fp != self.last_fingerprint {
            self.last_fingerprint = fp;
            self.change_count = self.change_count.saturating_add(1);
        }
        self.change_count
    }

    fn read_snapshot(&mut self) -> LocalClipboardSnapshot {
        self.read_snapshot_inner()
    }

    fn write_applied(&mut self, applied: &AppliedClipFfi) {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(err) => {
                eprintln!("sync-clip: clipboard open failed: {err}");
                return;
            }
        };
        if let (Some(bytes), Some(mime)) = (&applied.image_bytes, &applied.image_mime) {
            if mime.contains("png") {
                if let Some(image) = png_to_image_data(bytes) {
                    let _ = clipboard.set_image(image);
                }
            }
        }
        if !applied.text.is_empty() {
            let _ = clipboard.set_text(applied.text.clone());
        }
        let snapshot = LocalClipboardSnapshot {
            image_bytes: applied.image_bytes.clone(),
            image_mime: applied.image_mime.clone(),
            text: Some(applied.text.clone()),
        };
        self.last_fingerprint = Self::fingerprint(&snapshot);
        self.change_count = self.change_count.saturating_add(1);
    }
}

fn rgba_to_png(image: &ImageData<'_>) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    encoder
        .write_image(
            image.bytes.as_ref(),
            image.width as u32,
            image.height as u32,
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(out)
}

fn png_to_image_data(bytes: &[u8]) -> Option<ImageData<'static>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let width = img.width() as usize;
    let height = img.height() as usize;
    let bytes = img.into_raw();
    Some(ImageData {
        width,
        height,
        bytes: std::borrow::Cow::Owned(bytes),
    })
}
