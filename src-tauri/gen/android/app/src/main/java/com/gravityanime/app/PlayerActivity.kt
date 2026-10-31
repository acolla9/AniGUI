package com.gravityanime.app

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowManager
import android.widget.Button
import android.widget.FrameLayout
import androidx.annotation.OptIn
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.ui.PlayerView

@OptIn(UnstableApi::class)
class PlayerActivity : Activity() {
    private var player: ExoPlayer? = null
    private lateinit var request: PlayRequest
    private var completed = false
    private var failed = false
    private var message: String? = null
    private var resultSent = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        request = NativePlayback.requestFrom(intent)
        title = request.title
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        val view = PlayerView(this).apply {
            useController = true
            setShowSubtitleButton(true)
        }
        val root = FrameLayout(this).apply { setBackgroundColor(Color.BLACK) }
        root.addView(view, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))

        if (NativePlayback.cinemaAvailable()) {
            val btn = Button(this).apply {
                text = "Cinema (AR)"
                setOnClickListener { openCinema() }
            }
            val lp = FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.TOP or Gravity.END)
            lp.setMargins(24, 24, 24, 24)
            root.addView(btn, lp)
            view.setControllerVisibilityListener(
                PlayerView.ControllerVisibilityListener { v -> btn.visibility = v }
            )
        }
        setContentView(root)

        val p = ExoPlayer.Builder(this).build()
        player = p
        view.player = p
        p.setMediaSource(NativePlayback.buildSource(request), request.startMs)
        p.addListener(object : Player.Listener {
            override fun onPlaybackStateChanged(state: Int) {
                if (state == Player.STATE_ENDED) {
                    completed = true
                    sendResultAndFinish()
                }
            }

            override fun onPlayerError(error: PlaybackException) {
                failed = true
                message = "${error.errorCodeName}: ${error.message}"
                sendResultAndFinish()
            }
        })
        p.prepare()
        p.playWhenReady = true
    }

    private fun openCinema() {
        val pos = player?.currentPosition ?: 0L
        NativePlayback.setPrefersCinema(this, true)
        val cls = Class.forName(NativePlayback.CINEMA_CLASS)
        val i = NativePlayback.intentFor(this, cls, request.copy(startMs = pos))
        i.addFlags(Intent.FLAG_ACTIVITY_FORWARD_RESULT)
        player?.release()
        player = null
        resultSent = true
        startActivity(i)
        finish()
    }

    private fun sendResultAndFinish() {
        if (resultSent) return
        resultSent = true
        setResult(
            RESULT_OK,
            NativePlayback.resultIntent(completed, failed, message, player?.currentPosition ?: 0L)
        )
        finish()
    }

    override fun onStop() {
        super.onStop()
        player?.pause()
    }

    override fun onDestroy() {
        player?.release()
        player = null
        super.onDestroy()
    }
}