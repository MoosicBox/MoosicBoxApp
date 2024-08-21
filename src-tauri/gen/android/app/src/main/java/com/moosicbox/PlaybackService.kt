package com.moosicbox

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.graphics.Bitmap
import android.util.Log
import androidx.annotation.OptIn
import androidx.core.app.NotificationCompat
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import androidx.media3.ui.PlayerNotificationManager
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class PlaybackService : MediaSessionService() {
    private var mediaSession: MediaSession? = null
    private var player: Player? = null
    private lateinit var notificationManager: PlayerNotificationManager

    init {
        PlaybackService.instance = this
    }

    override fun onCreate() {
        super.onCreate()
        val player = MoosicBoxPlayer()
        this.player = player
        mediaSession =
                MediaSession.Builder(this, player).setCallback(MediaSessionCallback()).build()
    }

    // Remember to release the player and media session in onDestroy
    override fun onDestroy() {
        if (player?.isPlaying!!) {
            player?.stop()
        }
        notificationManager.setPlayer(null)
        player?.release()
        player = null
        stopSelf()

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

    fun initiateMediaNotification(player: Player) {
        val notificationManager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager

        val channelId = "moosicbox-notification-channel"
        val importance = NotificationManager.IMPORTANCE_HIGH
        val channel = NotificationChannel(channelId, "MoosicBox notifications", importance)
        channel.description = "MoosicBox notifications channel"

        // Register the channel with the system. You can't change the importance
        // or other notification behaviors after this.
        notificationManager.createNotificationChannel(channel)
        val playerNotificationManager =
                PlayerNotificationManager.Builder(this, 123, channelId)
                        .setSmallIconResourceId(R.mipmap.ic_launcher_round)
                        .setChannelImportance(importance)
                        .setChannelDescriptionResourceId(R.string.app_name)
                        .setChannelNameResourceId(R.string.app_name)
                        .setNotificationListener(notificationListener)
                        .build()

        Log.i("MOOSICBOX: PlaybackService", "setPlayer")

        Log.i(
                "MOOSICBOX: PlaybackService",
                "${player.getPlaybackState() == Player.STATE_IDLE} && ${player.isCommandAvailable(Player.COMMAND_GET_TIMELINE)} && ${player.getCurrentTimeline().isEmpty()}"
        )

        playerNotificationManager.setPlayer(player)
        playerNotificationManager.setPriority(NotificationCompat.PRIORITY_MAX)
        playerNotificationManager.setUseRewindAction(true)
        playerNotificationManager.setUseFastForwardAction(false)
        playerNotificationManager.setUsePreviousAction(false)
        playerNotificationManager.setUsePlayPauseActions(true)
    }

    private val notificationListener =
            @OptIn(UnstableApi::class)
            object : PlayerNotificationManager.NotificationListener {
                override fun onNotificationCancelled(
                        notificationId: Int,
                        dismissedByUser: Boolean
                ) {
                    super.onNotificationCancelled(notificationId, dismissedByUser)
                    if (player?.isPlaying!!) {
                        player?.stop()
                        player?.release()
                    }
                }

                override fun onNotificationPosted(
                        notificationId: Int,
                        notification: Notification,
                        ongoing: Boolean
                ) {
                    super.onNotificationPosted(notificationId, notification, ongoing)
                }
            }

    private val audioDescriptor =
            @OptIn(UnstableApi::class)
            object : PlayerNotificationManager.MediaDescriptionAdapter {
                override fun getCurrentContentTitle(player: Player): CharSequence {
                    return player.currentMediaItem?.mediaMetadata?.albumTitle!!
                }

                override fun createCurrentContentIntent(player: Player): PendingIntent? {
                    return pendingIntent()
                }

                override fun getCurrentContentText(player: Player): CharSequence? {
                    return ""
                }

                override fun getCurrentLargeIcon(
                        player: Player,
                        callback: PlayerNotificationManager.BitmapCallback
                ): Bitmap? {
                    return null
                }
            }

    private fun pendingIntent(): PendingIntent? {
        val intent = Intent(applicationContext, PlaybackService::class.java)
        return PendingIntent.getActivity(
                applicationContext,
                0,
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    @UnstableApi
    private inner class MediaSessionCallback : MediaSession.Callback {
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

        override fun onPlaybackResumption(
                mediaSession: MediaSession,
                controller: MediaSession.ControllerInfo
        ): ListenableFuture<MediaSession.MediaItemsWithStartPosition> {
            val settable = SettableFuture.create<MediaSession.MediaItemsWithStartPosition>()
            CoroutineScope(Dispatchers.Main).launch {
                val resumptionPlaylist =
                        MediaSession.MediaItemsWithStartPosition(mutableListOf(), 0, 0)
                settable.set(resumptionPlaylist)
            }
            return settable
        }
    }

    companion object {
        lateinit var instance: PlaybackService
    }
}
