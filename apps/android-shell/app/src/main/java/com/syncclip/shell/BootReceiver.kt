package com.syncclip.shell

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import uniffi.clip_ffi.LifetimeSnapshotFfi
import uniffi.clip_ffi.lifetimeBootShouldForcePaused
import uniffi.clip_ffi.lifetimeMayAutoStart

/**
 * Resume Shell Lifetime after device boot when Link Key + durable Armed (ADR-0006).
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(
        context: Context,
        intent: Intent?,
    ) {
        val action = intent?.action ?: return
        if (action != Intent.ACTION_BOOT_COMPLETED &&
            action != Intent.ACTION_LOCKED_BOOT_COMPLETED &&
            action != Intent.ACTION_MY_PACKAGE_REPLACED
        ) {
            return
        }
        val store = LinkKeyStore(context.applicationContext)
        val hasKey = store.load() != null
        val captureGranted = ElevatedClipboardCapture.isGranted(context)
        val snapshot =
            LifetimeSnapshotFfi(
                durableArmed = store.isArmed(),
                elevatedCaptureGranted = captureGranted,
                hasLinkKey = hasKey,
                quitOptedOut = false,
                requiresElevatedCapture = true,
            )
        if (lifetimeBootShouldForcePaused(snapshot)) {
            store.setArmed(false)
            ClipboardSyncService.startLifetime(context)
            // Notify why Armed could not resume (ADR-0006).
            val nm =
                context.getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
                nm.createNotificationChannel(
                    android.app.NotificationChannel(
                        "sync_clip_armed",
                        "Sync Clip",
                        android.app.NotificationManager.IMPORTANCE_LOW,
                    ),
                )
            }
            val notification =
                androidx.core.app.NotificationCompat.Builder(context, "sync_clip_armed")
                    .setContentTitle("Sync Clip Paused")
                    .setContentText("Elevated Clipboard Capture required after boot — open Sync Clip to Arm")
                    .setSmallIcon(android.R.drawable.ic_menu_share)
                    .setAutoCancel(true)
                    .build()
            nm.notify(43, notification)
            return
        }
        if (!lifetimeMayAutoStart(snapshot)) {
            return
        }
        ClipboardSyncService.startArmed(context)
    }
}
