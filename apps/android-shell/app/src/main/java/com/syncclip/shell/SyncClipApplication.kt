package com.syncclip.shell

import android.app.Application

/**
 * Android Shell application entry for Sync Clip.
 *
 * The Shell owns OS clipboard access, background lifetime, Link Key storage,
 * and UI. Sync behavior is delegated to the Clip Engine (linked later over FFI).
 */
class SyncClipApplication : Application()
