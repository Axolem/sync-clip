# Plain text only for Clip text in v1

Cross-OS rich text (HTML/RTF) paste is inconsistent and would dominate engine complexity. On capture, text is normalized to plain text; images remain a separate Clip part. Opaque “whatever the OS gave” blobs and best-effort rich round-trips are deferred until plain text + images prove the sync path.
