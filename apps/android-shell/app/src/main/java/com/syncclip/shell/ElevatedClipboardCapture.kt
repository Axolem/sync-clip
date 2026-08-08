package com.syncclip.shell

import android.accessibilityservice.AccessibilityServiceInfo
import android.content.Context
import android.provider.Settings
import android.view.accessibility.AccessibilityManager

/**
 * Android Elevated Clipboard Capture gate (ADR-0006).
 * Granted when this app's Accessibility service is enabled by the user.
 */
object ElevatedClipboardCapture {
    fun isGranted(context: Context): Boolean {
        val expected = "${context.packageName}/${ClipboardCaptureService::class.java.name}"
        val enabled =
            Settings.Secure.getString(
                context.contentResolver,
                Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES,
            ) ?: return false
        if (enabled.split(':').any { it.equals(expected, ignoreCase = true) }) {
            return true
        }
        // Fallback via AccessibilityManager flattened list.
        val am = context.getSystemService(Context.ACCESSIBILITY_SERVICE) as? AccessibilityManager
            ?: return false
        val enabledServices =
            am.getEnabledAccessibilityServiceList(AccessibilityServiceInfo.FEEDBACK_ALL_MASK)
                ?: return false
        return enabledServices.any { info ->
            info.resolveInfo?.serviceInfo?.let { si ->
                si.packageName == context.packageName &&
                    si.name == ClipboardCaptureService::class.java.name
            } == true
        }
    }
}
