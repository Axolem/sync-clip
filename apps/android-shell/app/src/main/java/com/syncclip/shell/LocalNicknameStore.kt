package com.syncclip.shell

import android.content.Context

/**
 * Optional Local Nickname stored only on this Device for UI (never on the wire).
 */
class LocalNicknameStore(context: Context) {
    private val prefs =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun load(): String? {
        val value = prefs.getString(KEY_NICKNAME, null)?.trim().orEmpty()
        return value.takeIf { it.isNotEmpty() }
    }

    fun save(nickname: String) {
        val trimmed = nickname.trim()
        if (trimmed.isEmpty()) {
            clear()
        } else {
            prefs.edit().putString(KEY_NICKNAME, trimmed).apply()
        }
    }

    fun clear() {
        prefs.edit().remove(KEY_NICKNAME).apply()
    }

    companion object {
        private const val KEY_NICKNAME = "local_nickname"
        private const val PREFS_NAME = "sync_clip_local_ui"
    }
}
