package com.syncclip.shell

import android.accessibilityservice.AccessibilityService
import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.view.accessibility.AccessibilityEvent
import java.util.concurrent.atomic.AtomicReference

/**
 * Elevated Clipboard Capture service (ADR-0006).
 * When enabled, observes clipboard changes from this privileged context and
 * publishes snapshots for [ClipboardSyncService] to sync while other apps are focused.
 */
class ClipboardCaptureService : AccessibilityService() {
    private val handler = Handler(Looper.getMainLooper())
    private var clipboard: ClipboardManager? = null

    private val listener =
        ClipboardManager.OnPrimaryClipChangedListener {
            capturePrimaryClip()
        }

    override fun onServiceConnected() {
        super.onServiceConnected()
        clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard?.addPrimaryClipChangedListener(listener)
        capturePrimaryClip()
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Window changes can coincide with copies on some OEMs — refresh opportunistically.
        if (event?.eventType == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) {
            handler.post { capturePrimaryClip() }
        }
    }

    override fun onInterrupt() {
        // No-op.
    }

    override fun onDestroy() {
        clipboard?.removePrimaryClipChangedListener(listener)
        clipboard = null
        super.onDestroy()
    }

    private fun capturePrimaryClip() {
        val cm = clipboard ?: return
        val clip = cm.primaryClip ?: return
        if (clip.itemCount < 1) return
        val item = clip.getItemAt(0)
        val text = item.coerceToText(this)?.toString()
        latest.set(
            CapturedClip(
                text = text,
                updatedAtMs = System.currentTimeMillis(),
            ),
        )
    }

    companion object {
        private val latest = AtomicReference<CapturedClip?>(null)

        fun consumeLatest(): CapturedClip? = latest.get()
    }
}

data class CapturedClip(
    val text: String?,
    val updatedAtMs: Long,
)
