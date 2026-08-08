package com.syncclip.shell

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import uniffi.clip_ffi.Session
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Foreground service that keeps clipboard sync alive while Armed.
 */
class ClipboardSyncService : Service() {
    private val echoGuard = ClipboardEchoGuard()
    private val handler = Handler(Looper.getMainLooper())
    private var lastPrimaryText: String? = null
    private val running = AtomicBoolean(false)
    private var session: Session? = null
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
        if (session != null) return
        session =
            Session(
                linkKeyBytes = credentials.linkKey,
                relayWsUrl = credentials.relayWsUrl,
                ephemeralIdBytes = credentials.ephemeralId,
            )
    }

    private fun startSyncLoop() {
        if (running.getAndSet(true)) return
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        lastPrimaryText = primaryText(clipboard)
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
        val text = primaryText(clipboard) ?: return
        if (text == lastPrimaryText) return
        lastPrimaryText = text
        if (echoGuard.shouldIgnoreChange(text)) return
        if (text.isEmpty()) return
        try {
            active.publishText(text)
        } catch (_: Exception) {
            // Paused or transient relay errors are ignored for the poll loop.
        }
    }

    private fun pollRemoteApplied() {
        val active = session ?: return
        if (!active.isArmed()) return
        val applied = active.pollApplied() ?: return
        echoGuard.markRemoteWrite(applied.text)
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(android.content.ClipData.newPlainText("sync-clip", applied.text))
        lastPrimaryText = applied.text
    }

    private fun primaryText(clipboard: ClipboardManager): String? {
        val clip = clipboard.primaryClip ?: return null
        if (clip.itemCount < 1) return null
        return clip.getItemAt(0).coerceToText(this)?.toString()
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
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Sync Clip Armed")
            .setContentText("Clipboard sync is active")
            .setSmallIcon(android.R.drawable.ic_menu_share)
            .setContentIntent(open)
            .setOngoing(true)
            .build()
    }

    companion object {
        const val ACTION_ARM = "com.syncclip.shell.action.ARM"
        const val ACTION_PAUSE = "com.syncclip.shell.action.PAUSE"
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
    }
}
