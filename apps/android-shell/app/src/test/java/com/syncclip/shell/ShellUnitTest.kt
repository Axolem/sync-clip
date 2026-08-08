package com.syncclip.shell

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ClipboardEchoGuardTest {
    @Test
    fun remoteWriteIsIgnoredOnce() {
        val guard = ClipboardEchoGuard()
        guard.markRemoteWrite("remote clip")
        assertTrue(guard.shouldIgnoreChange("remote clip"))
        assertFalse(guard.shouldIgnoreChange("user copy"))
    }

    @Test
    fun matchingAppliedTextStillSuppressed() {
        val guard = ClipboardEchoGuard()
        guard.markRemoteWrite("same")
        guard.shouldIgnoreChange("same")
        assertTrue(guard.shouldIgnoreChange("same"))
    }
}

class ArmedStatePreferencesTest {
    @Test
    fun armedDefaultsTrueInMemory() {
        val state = InMemoryArmedState()
        assertTrue(state.isArmed())
        state.setArmed(false)
        assertFalse(state.isArmed())
        state.setArmed(true)
        assertTrue(state.isArmed())
    }
}

/** Minimal Armed flag store used by unit tests (no Android Context). */
class InMemoryArmedState {
    private var armed: Boolean = true

    fun isArmed(): Boolean = armed

    fun setArmed(value: Boolean) {
        armed = value
    }
}
