package com.syncclip.shell

import android.os.Bundle
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

/**
 * Placeholder activity for the Android Shell.
 *
 * Sync Group joining via Link Key and Armed/Paused controls land in later slices.
 */
class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val label = TextView(this).apply {
            text = "sync-clip Android Shell — skeleton\n" +
                "Clip Engine not linked yet\n" +
                "Armed / Paused not implemented"
            textSize = 16f
            setPadding(48, 48, 48, 48)
        }
        setContentView(label)
    }
}
