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
import uniffi.clip_ffi.generateEphemeralId
import uniffi.clip_ffi.generateLinkKey
import uniffi.clip_ffi.linkKeyFromBase32
import uniffi.clip_ffi.linkKeyToBase32

/**
 * Android Shell UI: Link Key, Armed/Paused, relay URL.
 */
class MainActivity : AppCompatActivity() {
    private lateinit var armedSwitch: Switch
    private lateinit var linkKeyField: EditText
    private lateinit var relayField: EditText
    private lateinit var statusView: TextView
    private lateinit var store: LinkKeyStore

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = LinkKeyStore(this)
        requestNotificationPermissionIfNeeded()

        val root =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(48, 48, 48, 48)
            }

        statusView =
            TextView(this).apply {
                text = "Sync Clip Android Shell"
                textSize = 18f
            }
        root.addView(statusView)

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
                setText(LinkKeyStore.DEFAULT_RELAY)
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

    private fun restoreFields() {
        val credentials = store.load() ?: return
        linkKeyField.setText(linkKeyToBase32(credentials.linkKey))
        relayField.setText(credentials.relayWsUrl)
        armedSwitch.isChecked = store.isArmed()
        statusView.text = "Joined Sync Group (Link Key loaded)"
    }

    private fun onGenerate() {
        val key = generateLinkKey()
        val encoded = linkKeyToBase32(key)
        linkKeyField.setText(encoded)
        if (relayField.text.isNullOrBlank()) {
            relayField.setText(LinkKeyStore.DEFAULT_RELAY)
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
                    ?: LinkKeyStore.DEFAULT_RELAY
            val credentials =
                ShellCredentials(
                    ephemeralId = ephemeral,
                    linkKey = linkKey,
                    relayWsUrl = relay,
                )
            store.save(credentials)
            statusView.text = "Link Key saved"
            syncServiceWithArmedState()
            Toast.makeText(this, "Joined Sync Group", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Toast.makeText(this, "Invalid Link Key: ${e.message}", Toast.LENGTH_LONG).show()
        }
    }

    private fun onArmedChanged(armed: Boolean) {
        store.setArmed(armed)
        syncServiceWithArmedState()
    }

    private fun syncServiceWithArmedState() {
        if (store.load() == null) return
        if (store.isArmed()) {
            ClipboardSyncService.startArmed(this)
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
