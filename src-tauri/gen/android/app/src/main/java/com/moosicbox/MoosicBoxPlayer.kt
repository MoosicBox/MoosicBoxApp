package com.moosicbox

import android.net.Uri
import android.os.Looper
import android.util.Log
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import kotlin.collections.mutableListOf

@UnstableApi
class MoosicBoxPlayer : SimpleBasePlayer(Looper.getMainLooper()) {
    private var mediaItems: MutableList<MediaItem> = mutableListOf()

    private var permanentAvailableCommands: Player.Commands =
            Player.Commands.Builder()
                    .addAll(
                            COMMAND_PLAY_PAUSE,
                            COMMAND_PREPARE,
                            COMMAND_STOP,
                            COMMAND_SET_SPEED_AND_PITCH,
                            COMMAND_SET_SHUFFLE_MODE,
                            COMMAND_SET_REPEAT_MODE,
                            COMMAND_GET_CURRENT_MEDIA_ITEM,
                            COMMAND_GET_TIMELINE,
                            COMMAND_GET_METADATA,
                            COMMAND_SET_PLAYLIST_METADATA,
                            COMMAND_SET_MEDIA_ITEM,
                            COMMAND_CHANGE_MEDIA_ITEMS,
                            COMMAND_GET_TRACKS,
                            COMMAND_GET_AUDIO_ATTRIBUTES,
                            COMMAND_SET_AUDIO_ATTRIBUTES,
                            COMMAND_GET_VOLUME,
                            COMMAND_SET_VOLUME,
                            COMMAND_SET_VIDEO_SURFACE,
                            COMMAND_GET_TEXT,
                            COMMAND_RELEASE
                    )
                    .build()

    private var availableCommands: Player.Commands =
            Player.Commands.Builder()
                    .addAll(permanentAvailableCommands)
                    .add(COMMAND_SEEK_TO_DEFAULT_POSITION)
                    .add(COMMAND_SEEK_TO_MEDIA_ITEM)
                    .build()

    private var state: State = State.Builder().setAvailableCommands(availableCommands).build()

    init {
        MoosicBoxPlayer.player = this
    }

    override fun getState(): State {
        return this.state
    }

    override fun handleSetMediaItems(
            mediaItems: MutableList<MediaItem>,
            startIndex: Int,
            startPositionMs: Long
    ): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "setMediaItems $mediaItems $startIndex $startPositionMs")
        this.mediaItems = mediaItems
        this.state =
                state.buildUpon()
                        .setPlaylist(
                                mediaItems.map {
                                    MediaItemData.Builder(it.mediaId).setMediaItem(it).build()
                                }
                        )
                        .build()
        return Futures.immediateFuture(null)
    }

    override fun handlePrepare(): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "prepare")
        this.state = this.state.buildUpon().setPlaybackState(STATE_READY).build()
        return Futures.immediateFuture(null)
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "setPlayWhenReady $playWhenReady")
        this.state =
                this.state
                        .buildUpon()
                        .setPlayWhenReady(playWhenReady, PLAY_WHEN_READY_CHANGE_REASON_USER_REQUEST)
                        .build()
        return Futures.immediateFuture(null)
    }

    override fun handleStop(): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "stop")
        this.state =
                this.state
                        .buildUpon()
                        .setPlayWhenReady(false, PLAY_WHEN_READY_CHANGE_REASON_USER_REQUEST)
                        .setPlaybackState(STATE_ENDED)
                        .build()
        return Futures.immediateFuture(null)
    }

    override fun handleRelease(): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "release")
        return Futures.immediateFuture(null)
    }

    override fun handleSetRepeatMode(@Player.RepeatMode repeatMode: Int): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "setRepeatMode $repeatMode")
        this.state = this.state.buildUpon().setRepeatMode(repeatMode).build()
        return Futures.immediateFuture(null)
    }

    override fun handleSetShuffleModeEnabled(shuffleModeEnabled: Boolean): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "setShuffleModeEnabled $shuffleModeEnabled")
        this.state = this.state.buildUpon().setShuffleModeEnabled(shuffleModeEnabled).build()
        return Futures.immediateFuture(null)
    }

    override fun handleSetVideoOutput(videoOutput: Any): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "setVideoOutput $videoOutput")
        return Futures.immediateFuture(null)
    }

    override fun handleClearVideoOutput(videoOutput: Any?): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "clearVideoOutput $videoOutput")
        return Futures.immediateFuture(null)
    }

    companion object {
        lateinit var player: MoosicBoxPlayer

        init {
            com.moosicbox.playerplugin.Player.updateState = {
                Log.i("MoosicBoxPlayer", "Received state ${it}")

                it.playlist?.also {
                    val mediaItems =
                            it.tracks.map {
                                var metadataBuilder =
                                        MediaMetadata.Builder()
                                                .setArtist(it.artist)
                                                .setTitle(it.title)

                                it.albumCover?.also {
                                    metadataBuilder = metadataBuilder.setArtworkUri(Uri.parse(it))
                                }

                                val metadata = metadataBuilder.build()

                                MediaItem.Builder()
                                        .setMediaId("media-${it.id}")
                                        .setMediaMetadata(metadata)
                                        .build()
                            }

                    Log.i("MoosicBoxPlayer", "updateState mediaItems=${mediaItems}")
                    player.setMediaItems(mediaItems)
                    player.prepare()
                }
            }
        }
    }
}
