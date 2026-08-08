package com.syncclip.shell

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.View
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import com.google.android.material.button.MaterialButton
import com.google.android.material.switchmaterial.SwitchMaterial
import com.google.android.material.textfield.TextInputEditText
import uniffi.clip_ffi.defaultRelayWsUrl
import uniffi.clip_ffi.generateEphemeralId
import uniffi.clip_ffi.generateLinkKey
import uniffi.clip_ffi.linkKeyFromBase32
import uniffi.clip_ffi.linkKeyToBase32

/**
 * Android Shell UI: Link Key, Armed/Paused, relay URL, Local Nickname, rotation.
 */
class MainActivity : AppCompatActivity() {
    private lateinit var armedSwitch: SwitchMaterial
    private lateinit var linkKeyField: TextInputEditText
    private lateinit var nicknameField: TextInputEditText
    private lateinit var nicknameStore: LocalNicknameStore
    private lateinit var relayField: TextInputEditText
    private lateinit var statusView: TextView
    private lateinit var store: LinkKeyStore

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        store = LinkKeyStore(this)
        nicknameStore = LocalNicknameStore(this)
        requestNotificationPermissionIfNeeded()
        WindowCompat.setDecorFitsSystemWindows(window, true)
        setContentView(R.layout.activity_main)
        applySystemBarInsets(findViewById(R.id.rootScroll))

        statusView = findViewById(R.id.statusView)
        nicknameField = findViewById(R.id.nicknameField)
        linkKeyField = findViewById(R.id.linkKeyField)
        relayField = findViewById(R.id.relayField)
        armedSwitch = findViewById(R.id.armedSwitch)

        findViewById<MaterialButton>(R.id.saveNicknameButton).setOnClickListener { onSaveNickname() }
        findViewById<MaterialButton>(R.id.clearNicknameButton).setOnClickListener { onClearNickname() }
        findViewById<MaterialButton>(R.id.generateButton).setOnClickListener { onGenerate() }
        findViewById<MaterialButton>(R.id.saveJoinButton).setOnClickListener { onSaveJoin() }
        findViewById<MaterialButton>(R.id.rotateButton).setOnClickListener { onRotate() }

        armedSwitch.setOnCheckedChangeListener { _, checked -> onArmedChanged(checked) }

        restoreFields()
        syncServiceWithArmedState()
    }

    private fun applySystemBarInsets(root: View) {
        val initialLeft = root.paddingLeft
        val initialTop = root.paddingTop
        val initialRight = root.paddingRight
        val initialBottom = root.paddingBottom
        ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
            view.setPadding(
                initialLeft + bars.left,
                initialTop + bars.top,
                initialRight + bars.right,
                initialBottom + bars.bottom,
            )
            insets
        }
        ViewCompat.requestApplyInsets(root)
    }

    private fun titleLabel(): String {
        val nick = nicknameStore.load()
        return if (nick != null) "Sync Clip · $nick" else getString(R.string.status_ready)
    }

    private fun restoreFields() {
        nicknameField.setText(nicknameStore.load().orEmpty())
        statusView.text = titleLabel()
        relayField.setText(defaultRelayWsUrl())
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
            statusView.text = "${titleLabel()} — joined Sync Group"
            syncServiceWithArmedState()
            Toast.makeText(this, "Joined Sync Group", Toast.LENGTH_SHORT).show()
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
                if (encodedField.isNotEmpty() &&
                    existing != null &&
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
