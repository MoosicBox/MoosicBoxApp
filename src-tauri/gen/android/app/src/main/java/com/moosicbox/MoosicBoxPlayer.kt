package com.moosicbox

import android.os.Looper
import android.util.Log
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

    override fun getApplicationLooper(): Looper {
        Log.i("MoosicBoxPlayer", "getApplicationLooper")
        return Looper.getMainLooper()
    }

    override fun addListener(listener: Player.Listener) {
        Log.i("MoosicBoxPlayer", "addListener")
    }

    override fun removeListener(listener: Player.Listener) {
        Log.i("MoosicBoxPlayer", "removeListener")
    }

    override fun setMediaItems(mediaItems: MutableList<MediaItem>, resetPosition: Boolean) {
        Log.i("MoosicBoxPlayer", "setMediaItems")
        this.mediaItems = mediaItems
    }

    override fun setMediaItems(
            mediaItems: MutableList<MediaItem>,
            startIndex: Int,
            startPositionMs: Long
    ) {
        Log.i("MoosicBoxPlayer", "setMediaItems")
        this.mediaItems = mediaItems
    }

    override fun addMediaItems(index: Int, mediaItems: MutableList<MediaItem>) {
        Log.i("MoosicBoxPlayer", "addMediaItems")
        this.mediaItems.addAll(mediaItems)
    }

    override fun moveMediaItems(fromIndex: Int, toIndex: Int, newIndex: Int) {
        Log.i("MoosicBoxPlayer", "moveMediaItems")
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
        Log.i("MoosicBoxPlayer", "replaceMediaItems")
        for (x in fromIndex..toIndex) {
            this.mediaItems[x] = mediaItems[x - fromIndex]
        }
    }

    override fun removeMediaItems(fromIndex: Int, toIndex: Int) {
        Log.i("MoosicBoxPlayer", "removeMediaItems")
        for (x in fromIndex..toIndex) {
            this.mediaItems.removeAt(fromIndex)
        }
    }

    override fun getAvailableCommands(): Player.Commands {
        Log.i("MoosicBoxPlayer", "getAvailableCommands")
        return availableCommands
    }

    override fun prepare() {
        Log.i("MoosicBoxPlayer", "prepare")
    }

    override fun getPlaybackState(): Int {
        Log.i("MoosicBoxPlayer", "getPlaybackState")
        return Player.STATE_READY
    }

    override fun getPlaybackSuppressionReason(): Int {
        Log.i("MoosicBoxPlayer", "getPlaybackSuppressionReason")
        return Player.PLAYBACK_SUPPRESSION_REASON_NONE
    }

    override fun getPlayerError(): PlaybackException? {
        Log.i("MoosicBoxPlayer", "getPlayerError")
        return null
    }

    override fun setPlayWhenReady(playWhenReady: Boolean) {
        Log.i("MoosicBoxPlayer", "setPlayWhenReady")
    }

    override fun getPlayWhenReady(): Boolean {
        Log.i("MoosicBoxPlayer", "getPlayWhenReady")
        return false
    }

    override fun setRepeatMode(repeatMode: Int) {
        Log.i("MoosicBoxPlayer", "setRepeatMode")
    }

    override fun getRepeatMode(): Int {
        Log.i("MoosicBoxPlayer", "getRepeatMode")
        return Player.REPEAT_MODE_OFF
    }

    override fun setShuffleModeEnabled(shuffleModeEnabled: Boolean) {
        Log.i("MoosicBoxPlayer", "setShuffleModeEnabled")
    }

    override fun getShuffleModeEnabled(): Boolean {
        Log.i("MoosicBoxPlayer", "getShuffleModeEnabled")
        return false
    }

    override fun isLoading(): Boolean {
        Log.i("MoosicBoxPlayer", "isLoading")
        return false
    }

    override fun seekTo(
            mediaItemIndex: Int,
            positionMs: Long,
            seekCommand: Int,
            isRepeatingCurrentItem: Boolean
    ) {
        Log.i("MoosicBoxPlayer", "seekTo")
    }

    override fun getSeekBackIncrement(): Long {
        Log.i("MoosicBoxPlayer", "getSeekBackIncrement")
        return 0
    }

    override fun getSeekForwardIncrement(): Long {
        Log.i("MoosicBoxPlayer", "getSeekForwardIncrement")
        return 0
    }

    override fun getMaxSeekToPreviousPosition(): Long {
        Log.i("MoosicBoxPlayer", "getMaxSeekToPreviousPosition")
        return 0
    }

    override fun setPlaybackParameters(playbackParameters: PlaybackParameters) {
        Log.i("MoosicBoxPlayer", "setPlaybackParameters")
    }

    override fun getPlaybackParameters(): PlaybackParameters {
        Log.i("MoosicBoxPlayer", "getPlaybackParameters")
        return PlaybackParameters.DEFAULT
    }

    override fun stop() {
        Log.i("MoosicBoxPlayer", "stop")
    }

    override fun release() {
        Log.i("MoosicBoxPlayer", "release")
    }

    override fun getCurrentTracks(): Tracks {
        Log.i("MoosicBoxPlayer", "getCurrentTracks")
        return Tracks.EMPTY
    }

    override fun getTrackSelectionParameters(): TrackSelectionParameters {
        Log.i("MoosicBoxPlayer", "getTrackSelectionParameters")
        return TrackSelectionParameters.DEFAULT_WITHOUT_CONTEXT
    }

    override fun setTrackSelectionParameters(parameters: TrackSelectionParameters) {
        Log.i("MoosicBoxPlayer", "setTrackSelectionParameters")
    }

    override fun getMediaMetadata(): MediaMetadata {
        Log.i("MoosicBoxPlayer", "getMediaMetadata")
        return MediaMetadata.EMPTY
    }

    override fun getPlaylistMetadata(): MediaMetadata {
        Log.i("MoosicBoxPlayer", "getPlaylistMetadata")
        return MediaMetadata.EMPTY
    }

    override fun setPlaylistMetadata(mediaMetadata: MediaMetadata) {
        Log.i("MoosicBoxPlayer", "setPlaylistMetadata")
    }

    override fun getCurrentTimeline(): Timeline {
        Log.i("MoosicBoxPlayer", "getCurrentTimeline")
        return Timeline.EMPTY
    }

    override fun getCurrentPeriodIndex(): Int {
        Log.i("MoosicBoxPlayer", "getCurrentPeriodIndex")
        return 0
    }

    override fun getCurrentMediaItemIndex(): Int {
        Log.i("MoosicBoxPlayer", "getCurrentMediaItemIndex")
        return 0
    }

    override fun getDuration(): Long {
        Log.i("MoosicBoxPlayer", "getDuration")
        return 0
    }

    override fun getCurrentPosition(): Long {
        Log.i("MoosicBoxPlayer", "getCurrentPosition")
        return 0
    }

    override fun getBufferedPosition(): Long {
        Log.i("MoosicBoxPlayer", "getBufferedPosition")
        return 0
    }

    override fun getTotalBufferedDuration(): Long {
        Log.i("MoosicBoxPlayer", "getTotalBufferedDuration")
        return 0
    }

    override fun isPlayingAd(): Boolean {
        Log.i("MoosicBoxPlayer", "isPlayingAd")
        return false
    }

    override fun getCurrentAdGroupIndex(): Int {
        Log.i("MoosicBoxPlayer", "getCurrentAdGroupIndex")
        return -1
    }

    override fun getCurrentAdIndexInAdGroup(): Int {
        Log.i("MoosicBoxPlayer", "getCurrentAdIndexInAdGroup")
        return -1
    }

    override fun getContentPosition(): Long {
        Log.i("MoosicBoxPlayer", "getContentPosition")
        return 4
    }

    override fun getContentBufferedPosition(): Long {
        Log.i("MoosicBoxPlayer", "getContentBufferedPosition")
        return 0
    }

    override fun getAudioAttributes(): AudioAttributes {
        Log.i("MoosicBoxPlayer", "getAudioAttributes")
        return AudioAttributes.DEFAULT
    }

    override fun setVolume(volume: Float) {
        Log.i("MoosicBoxPlayer", "setVolume")
    }

    override fun getVolume(): Float {
        Log.i("MoosicBoxPlayer", "getVolume")
        return 1.0f
    }

    override fun clearVideoSurface() {
        Log.i("MoosicBoxPlayer", "clearVideoSurface")
    }

    override fun clearVideoSurface(surface: Surface?) {
        Log.i("MoosicBoxPlayer", "clearVideoSurface")
    }

    override fun setVideoSurface(surface: Surface?) {
        Log.i("MoosicBoxPlayer", "setVideoSurface")
    }

    override fun setVideoSurfaceHolder(surfaceHolder: SurfaceHolder?) {
        Log.i("MoosicBoxPlayer", "setVideoSurfaceHolder")
    }

    override fun clearVideoSurfaceHolder(surfaceHolder: SurfaceHolder?) {
        Log.i("MoosicBoxPlayer", "clearVideoSurfaceHolder")
    }

    override fun setVideoSurfaceView(surfaceView: SurfaceView?) {
        Log.i("MoosicBoxPlayer", "setVideoSurfaceView")
    }

    override fun clearVideoSurfaceView(surfaceView: SurfaceView?) {
        Log.i("MoosicBoxPlayer", "clearVideoSurfaceView")
    }

    override fun setVideoTextureView(textureView: TextureView?) {
        Log.i("MoosicBoxPlayer", "setVideoTextureView")
    }

    override fun clearVideoTextureView(textureView: TextureView?) {
        Log.i("MoosicBoxPlayer", "clearVideoTextureView")
    }

    override fun getVideoSize(): VideoSize {
        Log.i("MoosicBoxPlayer", "getVideoSize")
        return VideoSize.UNKNOWN
    }

    override fun getSurfaceSize(): Size {
        Log.i("MoosicBoxPlayer", "getSurfaceSize")
        return Size.ZERO
    }

    override fun getCurrentCues(): CueGroup {
        Log.i("MoosicBoxPlayer", "getCurrentCues")
        return CueGroup.EMPTY_TIME_ZERO
    }

    override fun getDeviceInfo(): DeviceInfo {
        Log.i("MoosicBoxPlayer", "getDeviceInfo")
        return DeviceInfo.UNKNOWN
    }

    override fun getDeviceVolume(): Int {
        Log.i("MoosicBoxPlayer", "getDeviceVolume")
        return 1
    }

    override fun isDeviceMuted(): Boolean {
        Log.i("MoosicBoxPlayer", "isDeviceMuted")
        return false
    }

    override fun setDeviceVolume(volume: Int) {
        Log.i("MoosicBoxPlayer", "setDeviceVolume")
    }

    override fun setDeviceVolume(volume: Int, flags: Int) {
        Log.i("MoosicBoxPlayer", "setDeviceVolume")
    }

    override fun increaseDeviceVolume() {
        Log.i("MoosicBoxPlayer", "increaseDeviceVolume")
    }

    override fun increaseDeviceVolume(flags: Int) {
        Log.i("MoosicBoxPlayer", "increaseDeviceVolume")
    }

    override fun decreaseDeviceVolume() {
        Log.i("MoosicBoxPlayer", "decreaseDeviceVolume")
    }

    override fun decreaseDeviceVolume(flags: Int) {
        Log.i("MoosicBoxPlayer", "decreaseDeviceVolume")
    }

    override fun setDeviceMuted(muted: Boolean) {
        Log.i("MoosicBoxPlayer", "setDeviceMuted")
    }

    override fun setDeviceMuted(muted: Boolean, flags: Int) {
        Log.i("MoosicBoxPlayer", "setDeviceMuted")
    }

    override fun setAudioAttributes(audioAttributes: AudioAttributes, handleAudioFocus: Boolean) {
        Log.i("MoosicBoxPlayer", "setAudioAttributes")
    }
}
