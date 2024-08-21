package com.moosicbox

import android.os.Looper
import android.util.Log
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import kotlin.collections.mutableListOf

@UnstableApi
class MoosicBoxPlayer : SimpleBasePlayer {
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

    private var state: SimpleBasePlayer.State =
            SimpleBasePlayer.State.Builder().setAvailableCommands(availableCommands).build()

    constructor() : super(Looper.getMainLooper()) {}

    override fun getState(): SimpleBasePlayer.State {
        return this.state
    }

    override fun handleSetMediaItems(
            mediaItems: MutableList<MediaItem>,
            startIndex: Int,
            startPositionMs: Long
    ): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "setMediaItems")
        this.mediaItems = mediaItems
        return Futures.immediateFuture(null)
    }

    override fun handlePrepare(): ListenableFuture<*> {
        Log.i("MoosicBoxPlayer", "prepare")
        return Futures.immediateFuture(null)
    }
}