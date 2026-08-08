package com.syncclip.shell

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
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

class InMemoryRelayUrlStoreTest {
    @Test
    fun relayUrlPersistsAcrossSaveLoad() {
        val store = InMemoryRelayCredentialsStore()
        assertNull(store.load())
        store.save(
            ShellCredentials(
                ephemeralId = ByteArray(16) { 1 },
                linkKey = ByteArray(32) { 2 },
                relayWsUrl = "ws://127.0.0.1:9999/v0/ws",
            ),
        )
        assertEquals("ws://127.0.0.1:9999/v0/ws", store.load()?.relayWsUrl)
        // Relaunch simulation: read again without rewriting.
        assertEquals("ws://127.0.0.1:9999/v0/ws", store.load()?.relayWsUrl)
    }

    @Test
    fun rotateClearsOldKeyThenSavesNew() {
        val store = InMemoryRelayCredentialsStore()
        val oldKey = ByteArray(32) { 9 }
        store.save(
            ShellCredentials(
                ephemeralId = ByteArray(16) { 1 },
                linkKey = oldKey,
                relayWsUrl = "ws://127.0.0.1:7120/v0/ws",
            ),
        )
        store.clear()
        assertNull(store.load())
        val newKey = ByteArray(32) { 8 }
        store.save(
            ShellCredentials(
                ephemeralId = ByteArray(16) { 1 },
                linkKey = newKey,
                relayWsUrl = "ws://127.0.0.1:7120/v0/ws",
            ),
        )
        assertTrue(store.load()!!.linkKey.contentEquals(newKey))
        assertFalse(store.load()!!.linkKey.contentEquals(oldKey))
    }
}

class InMemoryLocalNicknameStoreTest {
    @Test
    fun setClearPersistLocally() {
        val store = InMemoryLocalNicknameStore()
        assertNull(store.load())
        store.save("Phone")
        assertEquals("Phone", store.load())
        store.clear()
        assertNull(store.load())
    }

    @Test
    fun emptySaveClearsNickname() {
        val store = InMemoryLocalNicknameStore()
        store.save("temp")
        store.save("  ")
        assertNull(store.load())
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

/** In-memory stand-in for LinkKeyStore relay/credential persistence. */
class InMemoryRelayCredentialsStore {
    private var value: ShellCredentials? = null

    fun load(): ShellCredentials? = value

    fun save(credentials: ShellCredentials) {
        value = credentials
    }

    fun clear() {
        value = null
    }
}

/** In-memory stand-in for LocalNicknameStore. */
class InMemoryLocalNicknameStore {
    private var value: String? = null

    fun load(): String? = value

    fun save(nickname: String) {
        val trimmed = nickname.trim()
        value = if (trimmed.isEmpty()) null else trimmed
    }

    fun clear() {
        value = null
    }
}
