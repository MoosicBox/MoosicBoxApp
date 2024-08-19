package com.moosicbox

import androidx.annotation.OptIn
import androidx.media3.common.AudioAttributes
import androidx.media3.common.util.UnstableApi
import androidx.media3.common.C
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService

class PlaybackService : MediaSessionService() {
    private var mediaSession: MediaSession? = null

    override fun onCreate() {
        super.onCreate()
        val player =
                ExoPlayer.Builder(this)
                        .setAudioAttributes(AudioAttributes.DEFAULT, true)
                        .setHandleAudioBecomingNoisy(true)
                        .setWakeMode(C.WAKE_MODE_LOCAL)
                        .build()
        // Build the session with a custom layout.
        mediaSession =
                MediaSession.Builder(this, player).setCallback(MediaSessionCallback()).build()
    }

    // Remember to release the player and media session in onDestroy
    override fun onDestroy() {
        mediaSession?.run {
            player.release()
            release()
            mediaSession = null
        }
        super.onDestroy()
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? {
        return mediaSession
    }

    private inner class MediaSessionCallback : MediaSession.Callback {
        @OptIn(UnstableApi::class)
        override fun onConnect(
                session: MediaSession,
                controller: MediaSession.ControllerInfo
        ): MediaSession.ConnectionResult {
            // Set available player and session commands.
            return MediaSession.ConnectionResult.AcceptedResultBuilder(session)
                    .setAvailablePlayerCommands(
                            MediaSession.ConnectionResult.DEFAULT_PLAYER_COMMANDS
                                    .buildUpon()
                                    .build()
                    )
                    .setAvailableSessionCommands(
                            MediaSession.ConnectionResult.DEFAULT_SESSION_COMMANDS
                                    .buildUpon()
                                    .build()
                    )
                    .build()
        }
    }
}
