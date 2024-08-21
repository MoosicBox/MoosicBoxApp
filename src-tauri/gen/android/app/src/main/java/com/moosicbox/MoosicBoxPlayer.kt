package com.moosicbox

import android.os.Looper
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.TextureView
import androidx.media3.common.AudioAttributes
import androidx.media3.common.BasePlayer
import androidx.media3.common.DeviceInfo
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.Player
import androidx.media3.common.Timeline
import androidx.media3.common.TrackSelectionParameters
import androidx.media3.common.Tracks
import androidx.media3.common.VideoSize
import androidx.media3.common.text.CueGroup
import androidx.media3.common.util.Size
import androidx.media3.common.util.UnstableApi
import kotlin.collections.mutableListOf

@UnstableApi
class MoosicBoxPlayer : BasePlayer() {
    private var mediaItems: MutableList<MediaItem> = mutableListOf()

    override fun getApplicationLooper(): Looper {
        return Looper.getMainLooper()
    }

    override fun addListener(listener: Player.Listener) {}

    override fun removeListener(listener: Player.Listener) {}

    override fun setMediaItems(mediaItems: MutableList<MediaItem>, resetPosition: Boolean) {
        this.mediaItems = mediaItems
    }

    override fun setMediaItems(
            mediaItems: MutableList<MediaItem>,
            startIndex: Int,
            startPositionMs: Long
    ) {
        this.mediaItems = mediaItems
    }

    override fun addMediaItems(index: Int, mediaItems: MutableList<MediaItem>) {
        this.mediaItems.addAll(mediaItems)
    }

    override fun moveMediaItems(fromIndex: Int, toIndex: Int, newIndex: Int) {
        // Actually do it
        val old = this.mediaItems[fromIndex]
        this.mediaItems[fromIndex] = this.mediaItems[newIndex]
        this.mediaItems[newIndex] = old
    }

    override fun replaceMediaItems(
            fromIndex: Int,
            toIndex: Int,
            mediaItems: MutableList<MediaItem>
    ) {
        for (x in fromIndex..toIndex) {
            this.mediaItems[x] = mediaItems[x - fromIndex]
        }
    }

    override fun removeMediaItems(fromIndex: Int, toIndex: Int) {
        for (x in fromIndex..toIndex) {
            this.mediaItems.removeAt(fromIndex)
        }
    }

    override fun getAvailableCommands(): Player.Commands {
        return Player.Commands.EMPTY
    }

    override fun prepare() {}

    override fun getPlaybackState(): Int {
        return Player.STATE_READY
    }

    override fun getPlaybackSuppressionReason(): Int {
        return Player.PLAYBACK_SUPPRESSION_REASON_NONE
    }

    override fun getPlayerError(): PlaybackException? {
        return null
    }

    override fun setPlayWhenReady(playWhenReady: Boolean) {}

    override fun getPlayWhenReady(): Boolean {
        return false
    }

    override fun setRepeatMode(repeatMode: Int) {}

    override fun getRepeatMode(): Int {
        return Player.REPEAT_MODE_OFF
    }

    override fun setShuffleModeEnabled(shuffleModeEnabled: Boolean) {}

    override fun getShuffleModeEnabled(): Boolean {
        return false
    }

    override fun isLoading(): Boolean {
        return false
    }

    override fun seekTo(
            mediaItemIndex: Int,
            positionMs: Long,
            seekCommand: Int,
            isRepeatingCurrentItem: Boolean
    ) {}

    override fun getSeekBackIncrement(): Long {
        return 0
    }

    override fun getSeekForwardIncrement(): Long {
        return 0
    }

    override fun getMaxSeekToPreviousPosition(): Long {
        return 0
    }

    override fun setPlaybackParameters(playbackParameters: PlaybackParameters) {}

    override fun getPlaybackParameters(): PlaybackParameters {
        return PlaybackParameters.DEFAULT
    }

    override fun stop() {}

    override fun release() {}

    override fun getCurrentTracks(): Tracks {
        return Tracks.EMPTY
    }

    override fun getTrackSelectionParameters(): TrackSelectionParameters {
        return TrackSelectionParameters.DEFAULT_WITHOUT_CONTEXT
    }

    override fun setTrackSelectionParameters(parameters: TrackSelectionParameters) {}

    override fun getMediaMetadata(): MediaMetadata {
        return MediaMetadata.EMPTY
    }

    override fun getPlaylistMetadata(): MediaMetadata {
        return MediaMetadata.EMPTY
    }

    override fun setPlaylistMetadata(mediaMetadata: MediaMetadata) {}

    override fun getCurrentTimeline(): Timeline {
        return Timeline.EMPTY
    }

    override fun getCurrentPeriodIndex(): Int {
        return 0
    }

    override fun getCurrentMediaItemIndex(): Int {
        return 0
    }

    override fun getDuration(): Long {
        return 0
    }

    override fun getCurrentPosition(): Long {
        return 0
    }

    override fun getBufferedPosition(): Long {
        return 0
    }

    override fun getTotalBufferedDuration(): Long {
        return 0
    }

    override fun isPlayingAd(): Boolean {
        return false
    }

    override fun getCurrentAdGroupIndex(): Int {
        return -1
    }

    override fun getCurrentAdIndexInAdGroup(): Int {
        return -1
    }

    override fun getContentPosition(): Long {
        return 4
    }

    override fun getContentBufferedPosition(): Long {
        return 0
    }

    override fun getAudioAttributes(): AudioAttributes {
        return AudioAttributes.DEFAULT
    }

    override fun setVolume(volume: Float) {}

    override fun getVolume(): Float {
        return 1.0f
    }

    override fun clearVideoSurface() {}

    override fun clearVideoSurface(surface: Surface?) {}

    override fun setVideoSurface(surface: Surface?) {}

    override fun setVideoSurfaceHolder(surfaceHolder: SurfaceHolder?) {}

    override fun clearVideoSurfaceHolder(surfaceHolder: SurfaceHolder?) {}

    override fun setVideoSurfaceView(surfaceView: SurfaceView?) {}

    override fun clearVideoSurfaceView(surfaceView: SurfaceView?) {}

    override fun setVideoTextureView(textureView: TextureView?) {}

    override fun clearVideoTextureView(textureView: TextureView?) {}

    override fun getVideoSize(): VideoSize {
        return VideoSize.UNKNOWN
    }

    override fun getSurfaceSize(): Size {
        return Size.ZERO
    }

    override fun getCurrentCues(): CueGroup {
        return CueGroup.EMPTY_TIME_ZERO
    }

    override fun getDeviceInfo(): DeviceInfo {
        return DeviceInfo.UNKNOWN
    }

    override fun getDeviceVolume(): Int {
        return 1
    }

    override fun isDeviceMuted(): Boolean {
        return false
    }

    override fun setDeviceVolume(volume: Int) {}

    override fun setDeviceVolume(volume: Int, flags: Int) {}

    override fun increaseDeviceVolume() {}

    override fun increaseDeviceVolume(flags: Int) {}

    override fun decreaseDeviceVolume() {}

    override fun decreaseDeviceVolume(flags: Int) {}

    override fun setDeviceMuted(muted: Boolean) {}

    override fun setDeviceMuted(muted: Boolean, flags: Int) {}

    override fun setAudioAttributes(audioAttributes: AudioAttributes, handleAudioFocus: Boolean) {}
}
