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
import android.provider.Settings
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import uniffi.clip_ffi.AppliedClipFfi
import uniffi.clip_ffi.LifetimeSnapshotFfi
import uniffi.clip_ffi.Session
import uniffi.clip_ffi.lifetimeCaptureMissingShouldPersistPaused
import uniffi.clip_ffi.lifetimeMayEnterArmed
import uniffi.clip_ffi.lifetimeShouldKeepLifetime
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Foreground service that owns Shell Lifetime while a Link Key is saved (ADR-0006).
 * Pause keeps the service + relay session; only publish/accept stop.
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
                enforceCaptureGate()
                pollLocalClipboard()
                pollRemoteApplied()
                refreshNotificationIfNeeded()
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
                pauseKeepLifetime()
                return START_STICKY
            }
            ACTION_STOP_LIFETIME -> {
                stopLifetime()
                return START_NOT_STICKY
            }
            ACTION_ARM, ACTION_LIFETIME, null -> {
                startLifetime(armDesired = intent?.action != ACTION_LIFETIME)
            }
            ACTION_REJOIN -> {
                session?.close()
                session = null
                startLifetime(armDesired = store.isArmed())
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

    private fun startLifetime(armDesired: Boolean) {
        val credentials = store.load()
        if (credentials == null || !lifetimeShouldKeepLifetime(true)) {
            stopLifetime()
            return
        }
        if (armDesired) {
            tryArm()
        } else {
            store.setArmed(false)
        }
        ensureSession(credentials)
        session?.setArmed(store.isArmed())
        promoteForeground()
        startSyncLoop()
    }

    private fun tryArm(): Boolean {
        val snapshot =
            LifetimeSnapshotFfi(
                durableArmed = true,
                elevatedCaptureGranted = ElevatedClipboardCapture.isGranted(this),
                hasLinkKey = store.load() != null,
                quitOptedOut = false,
                requiresElevatedCapture = true,
            )
        if (!lifetimeMayEnterArmed(snapshot)) {
            store.setArmed(false)
            softFailReason = "Elevated Clipboard Capture required — enable Accessibility for Sync Clip"
            return false
        }
        store.setArmed(true)
        softFailReason = null
        return true
    }

    private fun enforceCaptureGate() {
        if (!store.isArmed()) return
        if (!lifetimeCaptureMissingShouldPersistPaused(
                true,
                ElevatedClipboardCapture.isGranted(this),
            )
        ) {
            return
        }
        store.setArmed(false)
        session?.setArmed(false)
        softFailReason = "Elevated Clipboard Capture revoked — Device Paused"
    }

    private fun pauseKeepLifetime() {
        store.setArmed(false)
        session?.setArmed(false)
        softFailReason = null
        promoteForeground()
    }

    private fun stopLifetime() {
        store.setArmed(false)
        stopSyncLoop()
        session?.close()
        session = null
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    private fun ensureSession(credentials: ShellCredentials) {
        if (session != null) return
        try {
            session =
                Session(
                    linkKeyBytes = credentials.linkKey,
                    relayWsUrl = credentials.relayWsUrl,
                    ephemeralIdBytes = credentials.ephemeralId,
                )
            softFailReason = softFailReason?.takeIf { it.contains("Capture") }
        } catch (e: Exception) {
            session = null
            softFailReason = e.message ?: "join failed"
            handler.postDelayed({
                val creds = store.load() ?: return@postDelayed
                session?.close()
                session = null
                ensureSession(creds)
                session?.setArmed(store.isArmed())
                promoteForeground()
            }, REJOIN_MS)
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
        var snapshot = readLocalSnapshot(clipboard)
        // Prefer Elevated Clipboard Capture bus when the OS withholds focus clipboard.
        if (snapshot.isEmpty()) {
            val captured = ClipboardCaptureService.consumeLatest()
            if (captured != null && !captured.text.isNullOrEmpty()) {
                snapshot = LocalClipboardSnapshot(text = captured.text)
            }
        }
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
            // Paused or Sync Idle publish errors are ignored for the poll loop.
        }
    }

    private fun pollRemoteApplied() {
        val active = session ?: return
        if (!active.isArmed()) return
        val applied = active.pollApplied() ?: return
        echoGuard.markRemoteWrite(applied.text)
        writeApplied(applied)
        lastFingerprint =
            fingerprint(
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
            val uri =
                writeCacheImage(imageBytes, mime) ?: run {
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

    private fun promoteForeground() {
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
    }

    private var lastNotificationBody: String? = null

    private fun refreshNotificationIfNeeded() {
        val body = notificationBody()
        if (body == lastNotificationBody) return
        lastNotificationBody = body
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, buildNotification())
    }

    private fun notificationBody(): String {
        val idle = session?.isSyncIdle() == true
        return when {
            softFailReason != null && softFailReason!!.contains("Paused") -> softFailReason!!
            softFailReason != null -> "Sync idle: $softFailReason"
            idle -> "Sync idle: reconnecting to relay"
            store.isArmed() -> "Clipboard sync is active"
            else -> "Paused — Shell Lifetime up"
        }
    }

    private fun buildNotification(): Notification {
        val open =
            PendingIntent.getActivity(
                this,
                0,
                Intent(this, MainActivity::class.java),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(
                if (store.isArmed()) {
                    "Sync Clip Armed"
                } else {
                    "Sync Clip Paused"
                },
            )
            .setContentText(notificationBody())
            .setSmallIcon(android.R.drawable.ic_menu_share)
            .setContentIntent(open)
            .setOngoing(true)
            .build()
    }

    companion object {
        const val ACTION_ARM = "com.syncclip.shell.action.ARM"
        const val ACTION_LIFETIME = "com.syncclip.shell.action.LIFETIME"
        const val ACTION_PAUSE = "com.syncclip.shell.action.PAUSE"
        const val ACTION_REJOIN = "com.syncclip.shell.action.REJOIN"
        const val ACTION_STOP_LIFETIME = "com.syncclip.shell.action.STOP_LIFETIME"
        private const val CHANNEL_ID = "sync_clip_armed"
        private const val NOTIFICATION_ID = 42
        private const val POLL_MS = 400L
        private const val REJOIN_MS = 2_000L

        fun startArmed(context: Context) {
            val intent = Intent(context, ClipboardSyncService::class.java).setAction(ACTION_ARM)
            ContextCompat.startForegroundService(context, intent)
        }

        fun startLifetime(context: Context) {
            val intent = Intent(context, ClipboardSyncService::class.java).setAction(ACTION_LIFETIME)
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

        fun stopLifetime(context: Context) {
            val intent =
                Intent(context, ClipboardSyncService::class.java).setAction(ACTION_STOP_LIFETIME)
            context.startService(intent)
        }

        fun openCaptureSettings(context: Context) {
            context.startActivity(
                Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
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
