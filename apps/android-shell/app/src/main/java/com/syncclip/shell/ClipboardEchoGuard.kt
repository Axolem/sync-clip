package com.syncclip.shell

/**
 * Suppresses echo when the Shell writes a remote Clip to ClipboardManager.
 */
class ClipboardEchoGuard {
    @Volatile
    private var ignoreRemaining: Int = 0

    @Volatile
    private var lastAppliedText: String? = null

    fun markRemoteWrite(text: String) {
        ignoreRemaining += 1
        lastAppliedText = text
    }

    fun shouldIgnoreChange(currentText: String?): Boolean {
        if (ignoreRemaining > 0) {
            ignoreRemaining -= 1
            return true
        }
        val applied = lastAppliedText
        if (currentText != null && applied != null && currentText == applied) {
            return true
        }
        return false
    }

    fun reset() {
        ignoreRemaining = 0
        lastAppliedText = null
    }
}
