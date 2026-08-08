package com.syncclip.shell

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import uniffi.clip_ffi.defaultRelayWsUrl

/**
 * Secure Link Key + ephemeral id + relay URL storage for the Android Shell.
 */
class LinkKeyStore(context: Context) {
    private val prefs: SharedPreferences

    init {
        val masterKey =
            MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
        prefs =
            EncryptedSharedPreferences.create(
                context,
                PREFS_NAME,
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
            )
    }

    fun load(): ShellCredentials? {
        val linkKeyB64 = prefs.getString(KEY_LINK_KEY, null) ?: return null
        val ephemeralB64 = prefs.getString(KEY_EPHEMERAL, null) ?: return null
        val relay = prefs.getString(KEY_RELAY, null) ?: defaultRelayWsUrl()
        return ShellCredentials(
            ephemeralId = android.util.Base64.decode(ephemeralB64, android.util.Base64.DEFAULT),
            linkKey = android.util.Base64.decode(linkKeyB64, android.util.Base64.DEFAULT),
            relayWsUrl = relay,
        )
    }

    fun save(credentials: ShellCredentials) {
        prefs.edit()
            .putString(
                KEY_EPHEMERAL,
                android.util.Base64.encodeToString(credentials.ephemeralId, android.util.Base64.DEFAULT),
            )
            .putString(
                KEY_LINK_KEY,
                android.util.Base64.encodeToString(credentials.linkKey, android.util.Base64.DEFAULT),
            )
            .putString(KEY_RELAY, credentials.relayWsUrl)
            .apply()
    }

    /** Clears the stored Link Key and related Sync Group credentials. */
    fun clear() {
        prefs.edit()
            .remove(KEY_LINK_KEY)
            .remove(KEY_EPHEMERAL)
            .remove(KEY_RELAY)
            .apply()
    }

    fun isArmed(): Boolean = prefs.getBoolean(KEY_ARMED, true)

    fun setArmed(armed: Boolean) {
        prefs.edit().putBoolean(KEY_ARMED, armed).apply()
    }

    companion object {
        private const val KEY_ARMED = "armed"
        private const val KEY_EPHEMERAL = "ephemeral_id"
        private const val KEY_LINK_KEY = "link_key"
        private const val KEY_RELAY = "relay_ws_url"
        private const val PREFS_NAME = "sync_clip_secure"
    }
}

data class ShellCredentials(
    val ephemeralId: ByteArray,
    val linkKey: ByteArray,
    val relayWsUrl: String,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is ShellCredentials) return false
        return ephemeralId.contentEquals(other.ephemeralId) &&
            linkKey.contentEquals(other.linkKey) &&
            relayWsUrl == other.relayWsUrl
    }

    override fun hashCode(): Int {
        var result = ephemeralId.contentHashCode()
        result = 31 * result + linkKey.contentHashCode()
        result = 31 * result + relayWsUrl.hashCode()
        return result
    }
}
