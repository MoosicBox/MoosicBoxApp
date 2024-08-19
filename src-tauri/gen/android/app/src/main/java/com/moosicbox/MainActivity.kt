package com.moosicbox

import android.content.ComponentName
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.google.common.util.concurrent.MoreExecutors

class MainActivity : TauriActivity() {
    override fun onStart() {
        val sessionToken = SessionToken(this, ComponentName(this, PlaybackService::class.java))
        val controllerFuture = MediaController.Builder(this, sessionToken).buildAsync()
        controllerFuture.addListener(
                {
                    // Call controllerFuture.get() to retrieve the MediaController.
                    // MediaController implements the Player interface, so it can be
                    // attached to the PlayerView UI component.
                    controllerFuture.get()
                },
                MoreExecutors.directExecutor()
        )
        super.onStart()
    }
}
