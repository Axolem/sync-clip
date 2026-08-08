package com.syncclip.shell

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.Switch
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import uniffi.clip_ffi.defaultRelayWsUrl
import uniffi.clip_ffi.generateEphemeralId
import uniffi.clip_ffi.generateLinkKey
import uniffi.clip_ffi.linkKeyFromBase32
import uniffi.clip_ffi.linkKeyToBase32

/**
 * Android Shell UI: Link Key, Armed/Paused, relay URL, Local Nickname, rotation.
 */
class MainActivity : AppCompatActivity() {
    private lateinit var armedSwitch: Switch
    private lateinit var linkKeyField: EditText
    private lateinit var nicknameField: EditText
    private lateinit var nicknameStore: LocalNicknameStore
    private lateinit var relayField: EditText
    private lateinit var statusView: TextView
    private lateinit var store: LinkKeyStore

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = LinkKeyStore(this)
        nicknameStore = LocalNicknameStore(this)
        requestNotificationPermissionIfNeeded()

        val root =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(48, 48, 48, 48)
            }

        statusView =
            TextView(this).apply {
                text = titleLabel()
                textSize = 18f
            }
        root.addView(statusView)

        nicknameField =
            EditText(this).apply {
                hint = "Local Nickname (this Device only)"
                setSingleLine()
            }
        root.addView(nicknameField)

        val saveNickname =
            Button(this).apply {
                text = "Save Local Nickname"
                setOnClickListener { onSaveNickname() }
            }
        root.addView(saveNickname)

        val clearNickname =
            Button(this).apply {
                text = "Clear Local Nickname"
                setOnClickListener { onClearNickname() }
            }
        root.addView(clearNickname)

        linkKeyField =
            EditText(this).apply {
                hint = "Link Key (base32)"
                setSingleLine()
            }
        root.addView(linkKeyField)

        relayField =
            EditText(this).apply {
                hint = "Relay WebSocket URL"
                setSingleLine()
                setText(defaultRelayWsUrl())
            }
        root.addView(relayField)

        val generate =
            Button(this).apply {
                text = "Generate Link Key"
                setOnClickListener { onGenerate() }
            }
        root.addView(generate)

        val saveJoin =
            Button(this).apply {
                text = "Save / Join"
                setOnClickListener { onSaveJoin() }
            }
        root.addView(saveJoin)

        val rotate =
            Button(this).apply {
                text = "Rotate Link Key"
                setOnClickListener { onRotate() }
            }
        root.addView(rotate)

        armedSwitch =
            Switch(this).apply {
                text = "Armed"
                isChecked = store.isArmed()
                setOnCheckedChangeListener { _, checked -> onArmedChanged(checked) }
            }
        root.addView(armedSwitch)

        setContentView(root)
        restoreFields()
        syncServiceWithArmedState()
    }

    private fun titleLabel(): String {
        val nick = nicknameStore.load()
        return if (nick != null) "Sync Clip · $nick" else "Sync Clip Android Shell"
    }

    private fun restoreFields() {
        nicknameField.setText(nicknameStore.load().orEmpty())
        statusView.text = titleLabel()
        val credentials = store.load() ?: return
        linkKeyField.setText(linkKeyToBase32(credentials.linkKey))
        relayField.setText(credentials.relayWsUrl)
        armedSwitch.isChecked = store.isArmed()
        statusView.text = "${titleLabel()} — Link Key loaded"
    }

    private fun onSaveNickname() {
        nicknameStore.save(nicknameField.text?.toString().orEmpty())
        statusView.text = titleLabel()
        Toast.makeText(this, "Local Nickname saved (local only)", Toast.LENGTH_SHORT).show()
    }

    private fun onClearNickname() {
        nicknameStore.clear()
        nicknameField.setText("")
        statusView.text = titleLabel()
        Toast.makeText(this, "Local Nickname cleared", Toast.LENGTH_SHORT).show()
    }

    private fun onGenerate() {
        val key = generateLinkKey()
        val encoded = linkKeyToBase32(key)
        linkKeyField.setText(encoded)
        if (relayField.text.isNullOrBlank()) {
            relayField.setText(defaultRelayWsUrl())
        }
        Toast.makeText(this, "Link Key generated — tap Save / Join", Toast.LENGTH_SHORT).show()
    }

    private fun onSaveJoin() {
        try {
            val encoded = linkKeyField.text?.toString()?.trim().orEmpty()
            if (encoded.isEmpty()) {
                Toast.makeText(this, "Enter or generate a Link Key", Toast.LENGTH_SHORT).show()
                return
            }
            val linkKey = linkKeyFromBase32(encoded)
            val existing = store.load()
            val ephemeral = existing?.ephemeralId ?: generateEphemeralId()
            val relay =
                relayField.text?.toString()?.trim().takeUnless { it.isNullOrEmpty() }
                    ?: defaultRelayWsUrl()
            val credentials =
                ShellCredentials(
                    ephemeralId = ephemeral,
                    linkKey = linkKey,
                    relayWsUrl = relay,
                )
            store.save(credentials)
            statusView.text = "${titleLabel()} — Link Key saved"
            syncServiceWithArmedState()
            Toast.makeText(this, "Joined Sync Group (or sync idle if relay unreachable)", Toast.LENGTH_SHORT)
                .show()
        } catch (e: Exception) {
            statusView.text = "${titleLabel()} — sync idle: ${e.message}"
            Toast.makeText(this, "Join soft-fail: ${e.message}", Toast.LENGTH_LONG).show()
        }
    }

    private fun onRotate() {
        try {
            val existing = store.load()
            val ephemeral = existing?.ephemeralId ?: generateEphemeralId()
            val relay =
                relayField.text?.toString()?.trim().takeUnless { it.isNullOrEmpty() }
                    ?: existing?.relayWsUrl
                    ?: defaultRelayWsUrl()
            val encodedField = linkKeyField.text?.toString()?.trim().orEmpty()
            val newKey =
                if (encodedField.isNotEmpty() && existing != null &&
                    encodedField != linkKeyToBase32(existing.linkKey)
                ) {
                    linkKeyFromBase32(encodedField)
                } else {
                    generateLinkKey()
                }
            store.clear()
            val credentials =
                ShellCredentials(
                    ephemeralId = ephemeral,
                    linkKey = newKey,
                    relayWsUrl = relay,
                )
            store.save(credentials)
            linkKeyField.setText(linkKeyToBase32(newKey))
            relayField.setText(relay)
            statusView.text = "${titleLabel()} — Link Key rotated"
            syncServiceWithArmedState()
            Toast.makeText(this, "Rotated Link Key", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            statusView.text = "${titleLabel()} — rotate soft-fail: ${e.message}"
            Toast.makeText(this, "Rotate soft-fail: ${e.message}", Toast.LENGTH_LONG).show()
        }
    }

    private fun onArmedChanged(armed: Boolean) {
        store.setArmed(armed)
        syncServiceWithArmedState()
    }

    private fun syncServiceWithArmedState() {
        if (store.load() == null) return
        if (store.isArmed()) {
            ClipboardSyncService.rejoin(this)
        } else {
            ClipboardSyncService.pause(this)
        }
    }

    private fun requestNotificationPermissionIfNeeded() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        ActivityCompat.requestPermissions(
            this,
            arrayOf(Manifest.permission.POST_NOTIFICATIONS),
            1001,
        )
    }
}
