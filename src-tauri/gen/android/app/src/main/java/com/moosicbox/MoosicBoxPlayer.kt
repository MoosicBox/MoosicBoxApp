package com.moosicbox

import android.os.Looper
import android.util.Log
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.TextureView
import androidx.media3.common.AudioAttributes
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
class MoosicBoxPlayer : Player {
    private var mediaItems: MutableList<MediaItem> = mutableListOf()

    override fun getApplicationLooper(): Looper {
        return Looper.getMainLooper()
    }

    override fun addListener(listener: Player.Listener) {}

    override fun removeListener(listener: Player.Listener) {}

    override fun setMediaItems(mediaItems: MutableList<MediaItem>) {
        Log.i("MyPlayer", "setMediaItems len=${mediaItems.count()}")
        this.mediaItems = mediaItems
    }

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

    override fun setMediaItem(mediaItem: MediaItem) {
        this.mediaItems = mutableListOf(mediaItem)
    }

    override fun setMediaItem(mediaItem: MediaItem, startPositionMs: Long) {
        this.mediaItems = mutableListOf(mediaItem)
    }

    override fun setMediaItem(mediaItem: MediaItem, resetPosition: Boolean) {
        this.mediaItems = mutableListOf(mediaItem)
    }

    override fun addMediaItem(mediaItem: MediaItem) {
        this.mediaItems.add(mediaItem)
    }

    override fun addMediaItem(index: Int, mediaItem: MediaItem) {
        this.mediaItems.add(index, mediaItem)
    }

    override fun addMediaItems(mediaItems: MutableList<MediaItem>) {
        this.mediaItems.addAll(mediaItems)
    }

    override fun addMediaItems(index: Int, mediaItems: MutableList<MediaItem>) {
        this.mediaItems.addAll(mediaItems)
    }

    override fun moveMediaItem(currentIndex: Int, newIndex: Int) {
        val old = this.mediaItems[currentIndex]
        this.mediaItems[currentIndex] = this.mediaItems[newIndex]
        this.mediaItems[newIndex] = old
    }

    override fun moveMediaItems(fromIndex: Int, toIndex: Int, newIndex: Int) {
        // Actually do it
        val old = this.mediaItems[fromIndex]
        this.mediaItems[fromIndex] = this.mediaItems[newIndex]
        this.mediaItems[newIndex] = old
    }

    override fun replaceMediaItem(index: Int, mediaItem: MediaItem) {
        this.mediaItems[index] = mediaItem
    }

    override fun replaceMediaItems(
            fromIndex: Int,
            toIndex: Int,
            mediaItems: MutableList<MediaItem>
    ) {
        this.mediaItems[fromIndex] = mediaItems[0]
    }

    override fun removeMediaItem(index: Int) {
        this.mediaItems.removeAt(index)
    }

    override fun removeMediaItems(fromIndex: Int, toIndex: Int) {
        for (x in fromIndex..toIndex) {
            this.mediaItems.removeAt(fromIndex)
        }
    }

    override fun clearMediaItems() {
        this.mediaItems.clear()
    }

    override fun isCommandAvailable(command: Int): Boolean {
        return true
    }

    override fun canAdvertiseSession(): Boolean {
        return true
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

    override fun isPlaying(): Boolean {
        return false
    }

    override fun getPlayerError(): PlaybackException? {
        return null
    }

    override fun play() {}

    override fun pause() {}

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

    override fun seekToDefaultPosition() {}

    override fun seekToDefaultPosition(mediaItemIndex: Int) {}

    override fun seekTo(positionMs: Long) {}

    override fun seekTo(mediaItemIndex: Int, positionMs: Long) {}

    override fun getSeekBackIncrement(): Long {
        return 0
    }

    override fun seekBack() {}

    override fun getSeekForwardIncrement(): Long {
        return 0
    }

    override fun seekForward() {}

    override fun hasPrevious(): Boolean {
        return false
    }

    override fun hasPreviousWindow(): Boolean {
        return false
    }

    override fun hasPreviousMediaItem(): Boolean {
        return getPreviousMediaItemIndex() >= 0
    }

    override fun previous() {}

    override fun seekToPreviousWindow() {}

    override fun seekToPreviousMediaItem() {}

    override fun getMaxSeekToPreviousPosition(): Long {
        return 0
    }

    override fun seekToPrevious() {}

    override fun hasNext(): Boolean {
        return false
    }

    override fun hasNextWindow(): Boolean {
        return false
    }

    override fun hasNextMediaItem(): Boolean {
        return getCurrentMediaItemIndex() + 1 < mediaItems.count()
    }

    override fun next() {}

    override fun seekToNextWindow() {}

    override fun seekToNextMediaItem() {}

    override fun seekToNext() {}

    override fun setPlaybackParameters(playbackParameters: PlaybackParameters) {}

    override fun setPlaybackSpeed(speed: Float) {}

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

    override fun getCurrentManifest(): Any? {
        return null
    }

    override fun getCurrentTimeline(): Timeline {
        return Timeline.EMPTY
    }

    override fun getCurrentPeriodIndex(): Int {
        return 0
    }

    override fun getCurrentWindowIndex(): Int {
        return 0
    }

    override fun getCurrentMediaItemIndex(): Int {
        return 0
    }

    override fun getNextWindowIndex(): Int {
        return 0
    }

    override fun getNextMediaItemIndex(): Int {
        return getCurrentMediaItemIndex() + 1
    }

    override fun getPreviousWindowIndex(): Int {
        return 0
    }

    override fun getPreviousMediaItemIndex(): Int {
        return getCurrentMediaItemIndex() - 1
    }

    override fun getCurrentMediaItem(): MediaItem? {
        Log.i(
                "MyPlayer",
                "getCurrentMediaItem index=${getCurrentMediaItemIndex()} len=${mediaItems.count()}"
        )
        if (getCurrentMediaItemIndex() < mediaItems.count()) {
            return mediaItems[getCurrentMediaItemIndex()]
        }
        return null
    }

    override fun getMediaItemCount(): Int {
        return mediaItems.count()
    }

    override fun getMediaItemAt(index: Int): MediaItem {
        return mediaItems[index]
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

    override fun getBufferedPercentage(): Int {
        return 0
    }

    override fun getTotalBufferedDuration(): Long {
        return 0
    }

    override fun isCurrentWindowDynamic(): Boolean {
        return false
    }

    override fun isCurrentMediaItemDynamic(): Boolean {
        return false
    }

    override fun isCurrentWindowLive(): Boolean {
        return true
    }

    override fun isCurrentMediaItemLive(): Boolean {
        return true
    }

    override fun getCurrentLiveOffset(): Long {
        return 0
    }

    override fun isCurrentWindowSeekable(): Boolean {
        return true
    }

    override fun isCurrentMediaItemSeekable(): Boolean {
        return true
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

    override fun getContentDuration(): Long {
        return 10
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
