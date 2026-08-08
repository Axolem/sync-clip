package com.syncclip.shell

import android.app.Application

/**
 * Android Shell application entry for Sync Clip.
 *
 * Owns OS clipboard access, background lifetime, Link Key storage, and UI.
 * Sync behavior is delegated to the Clip Engine over UniFFI (`clip-ffi`).
 */
class SyncClipApplication : Application()
