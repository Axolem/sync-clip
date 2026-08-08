package com.syncclip.shell

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import uniffi.clip_ffi.AppliedClipFfi
import uniffi.clip_ffi.Session
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Foreground service that keeps clipboard sync alive while Armed.
 */
class ClipboardSyncService : Service() {
    private val echoGuard = ClipboardEchoGuard()
    private val handler = Handler(Looper.getMainLooper())
    private var lastFingerprint: String? = null
    private val running = AtomicBoolean(false)
    private var session: Session? = null
    private var softFailReason: String? = null
    private lateinit var store: LinkKeyStore

    private val tick =
        object : Runnable {
            override fun run() {
                if (!running.get()) return
                pollLocalClipboard()
                pollRemoteApplied()
                handler.postDelayed(this, POLL_MS)
            }
        }

    override fun onCreate() {
        super.onCreate()
        store = LinkKeyStore(applicationContext)
        createChannel()
    }

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        when (intent?.action) {
            ACTION_PAUSE -> {
                pauseAndStop()
                return START_NOT_STICKY
            }
            ACTION_ARM, null -> {
                startArmed()
            }
            ACTION_REJOIN -> {
                session?.close()
                session = null
                startArmed()
            }
        }
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        stopSyncLoop()
        session?.close()
        session = null
        super.onDestroy()
    }

    private fun startArmed() {
        val credentials = store.load()
        if (credentials == null) {
            stopSelf()
            return
        }
        store.setArmed(true)
        ensureSession(credentials)
        session?.setArmed(true)
        val notification = buildNotification()
        val fgsType =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
            } else {
                0
            }
        ServiceCompat.startForeground(
            this,
            NOTIFICATION_ID,
            notification,
            fgsType,
        )
        startSyncLoop()
    }

    private fun pauseAndStop() {
        store.setArmed(false)
        session?.setArmed(false)
        stopSyncLoop()
        session?.close()
        session = null
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun ensureSession(credentials: ShellCredentials) {
        session?.close()
        session = null
        try {
            session =
                Session(
                    linkKeyBytes = credentials.linkKey,
                    relayWsUrl = credentials.relayWsUrl,
                    ephemeralIdBytes = credentials.ephemeralId,
                )
            softFailReason = null
        } catch (e: Exception) {
            // Soft fail: stay Armed (notification) but sync idle without crashing.
            session = null
            softFailReason = e.message ?: "join failed"
        }
    }

    private fun startSyncLoop() {
        if (running.getAndSet(true)) return
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        lastFingerprint = fingerprint(readLocalSnapshot(clipboard))
        handler.post(tick)
    }

    private fun stopSyncLoop() {
        running.set(false)
        handler.removeCallbacks(tick)
    }

    private fun pollLocalClipboard() {
        val active = session ?: return
        if (!active.isArmed()) return
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val snapshot = readLocalSnapshot(clipboard)
        val fp = fingerprint(snapshot)
        if (fp == lastFingerprint) return
        lastFingerprint = fp
        if (echoGuard.shouldIgnoreChange(snapshot.text)) return
        if (snapshot.isEmpty()) return
        try {
            val text = snapshot.text.orEmpty()
            val imageBytes = snapshot.imageBytes
            val mime = snapshot.imageMime
            if (imageBytes != null && mime != null) {
                active.publishTextAndImage(text, imageBytes, mime)
            } else if (text.isNotEmpty()) {
                active.publishText(text)
            }
        } catch (_: Exception) {
            // Paused or transient relay errors are ignored for the poll loop.
        }
    }

    private fun pollRemoteApplied() {
        val active = session ?: return
        if (!active.isArmed()) return
        val applied = active.pollApplied() ?: return
        echoGuard.markRemoteWrite(applied.text)
        writeApplied(applied)
        lastFingerprint = fingerprint(
            LocalClipboardSnapshot(
                imageBytes = applied.imageBytes,
                imageMime = applied.imageMime,
                text = applied.text.takeIf { it.isNotEmpty() },
            ),
        )
    }

    private fun readLocalSnapshot(clipboard: ClipboardManager): LocalClipboardSnapshot {
        val clip = clipboard.primaryClip ?: return LocalClipboardSnapshot()
        if (clip.itemCount < 1) return LocalClipboardSnapshot()
        val item = clip.getItemAt(0)
        val text = item.coerceToText(this)?.toString()
        val uri = item.uri
        if (uri != null) {
            val mime = contentResolver.getType(uri) ?: guessImageMime(clip)
            if (mime != null && mime.startsWith("image/")) {
                val bytes = readUriBytes(uri)
                if (bytes != null) {
                    return LocalClipboardSnapshot(imageBytes = bytes, imageMime = mime, text = text)
                }
            }
        }
        return LocalClipboardSnapshot(text = text)
    }

    private fun guessImageMime(clip: ClipData): String? {
        val desc = clip.description ?: return null
        for (i in 0 until desc.mimeTypeCount) {
            val mime = desc.getMimeType(i)
            if (mime.startsWith("image/")) return mime
        }
        return null
    }

    private fun readUriBytes(uri: Uri): ByteArray? =
        try {
            contentResolver.openInputStream(uri)?.use { it.readBytes() }
        } catch (_: Exception) {
            null
        }

    private fun writeApplied(applied: AppliedClipFfi) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val imageBytes = applied.imageBytes
        val mime = applied.imageMime ?: "image/png"
        if (imageBytes != null && imageBytes.isNotEmpty()) {
            val uri = writeCacheImage(imageBytes, mime) ?: run {
                clipboard.setPrimaryClip(ClipData.newPlainText("sync-clip", applied.text))
                return
            }
            val clip =
                if (applied.text.isNotEmpty()) {
                    ClipData.newUri(contentResolver, "sync-clip", uri).also {
                        it.addItem(ClipData.Item(applied.text))
                    }
                } else {
                    ClipData.newUri(contentResolver, "sync-clip", uri)
                }
            clipboard.setPrimaryClip(clip)
            return
        }
        clipboard.setPrimaryClip(ClipData.newPlainText("sync-clip", applied.text))
    }

    private fun writeCacheImage(
        bytes: ByteArray,
        mime: String,
    ): Uri? =
        try {
            val ext =
                when {
                    mime.contains("jpeg") || mime.contains("jpg") -> "jpg"
                    mime.contains("webp") -> "webp"
                    else -> "png"
                }
            val dir = File(cacheDir, "clipboard").apply { mkdirs() }
            val file = File(dir, "applied.$ext")
            file.writeBytes(bytes)
            FileProvider.getUriForFile(this, "$packageName.fileprovider", file)
        } catch (_: Exception) {
            null
        }

    private fun fingerprint(snapshot: LocalClipboardSnapshot): String =
        buildString {
            append(snapshot.text.orEmpty())
            append('|')
            append(snapshot.imageMime.orEmpty())
            append('|')
            append(snapshot.imageBytes?.size ?: 0)
        }

    private fun createChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = getSystemService(NotificationManager::class.java)
        val channel =
            NotificationChannel(
                CHANNEL_ID,
                "Sync Clip",
                NotificationManager.IMPORTANCE_LOW,
            )
        manager.createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification {
        val open =
            PendingIntent.getActivity(
                this,
                0,
                Intent(this, MainActivity::class.java),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
        val body =
            softFailReason?.let { "Sync idle: $it" } ?: "Clipboard sync is active"
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Sync Clip Armed")
            .setContentText(body)
            .setSmallIcon(android.R.drawable.ic_menu_share)
            .setContentIntent(open)
            .setOngoing(true)
            .build()
    }

    companion object {
        const val ACTION_ARM = "com.syncclip.shell.action.ARM"
        const val ACTION_PAUSE = "com.syncclip.shell.action.PAUSE"
        const val ACTION_REJOIN = "com.syncclip.shell.action.REJOIN"
        private const val CHANNEL_ID = "sync_clip_armed"
        private const val NOTIFICATION_ID = 42
        private const val POLL_MS = 400L

        fun startArmed(context: Context) {
            val intent = Intent(context, ClipboardSyncService::class.java).setAction(ACTION_ARM)
            ContextCompat.startForegroundService(context, intent)
        }

        fun pause(context: Context) {
            val intent = Intent(context, ClipboardSyncService::class.java).setAction(ACTION_PAUSE)
            context.startService(intent)
        }

        fun rejoin(context: Context) {
            val intent = Intent(context, ClipboardSyncService::class.java).setAction(ACTION_REJOIN)
            ContextCompat.startForegroundService(context, intent)
        }
    }
}

data class LocalClipboardSnapshot(
    val imageBytes: ByteArray? = null,
    val imageMime: String? = null,
    val text: String? = null,
) {
    fun isEmpty(): Boolean {
        val textEmpty = text.isNullOrEmpty()
        val imageEmpty = imageBytes == null || imageBytes.isEmpty()
        return textEmpty && imageEmpty
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is LocalClipboardSnapshot) return false
        return text == other.text &&
            imageMime == other.imageMime &&
            imageBytes.contentEquals(other.imageBytes)
    }

    override fun hashCode(): Int {
        var result = text?.hashCode() ?: 0
        result = 31 * result + (imageMime?.hashCode() ?: 0)
        result = 31 * result + (imageBytes?.contentHashCode() ?: 0)
        return result
    }
}
