package com.moosicbox.playerplugin

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke

data class Track (
  val id: Int = 0,
  val title: String = ""
)

data class Playlist (
  val tracks: List<Track> = listOf()
)

@InvokeArg
data class State (
  val playlist: Playlist? = null
)

@TauriPlugin
class PlayerPlugin(private val activity: Activity): Plugin(activity) {
    private val implementation = Player()

    @Command
    fun updateState(invoke: Invoke) {
        val args = invoke.parseArgs(State::class.java)

        val ret = JSObject()
        implementation.updateState(args)
        invoke.resolve(ret)
    }
}
