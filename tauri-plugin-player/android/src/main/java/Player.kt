package com.moosicbox.playerplugin

import android.util.Log

class Player {
    fun updateState(state: State) {
        Player.updateState(state)
    }

    companion object {
        public lateinit var updateState: (State) -> Unit
    }
}
