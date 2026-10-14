package dev.incant.player;

import android.app.Activity;
import android.media.AudioAttributes;
import android.media.AudioFocusRequest;
import android.media.AudioManager;
import android.os.Bundle;
import android.util.Log;
import android.view.Choreographer;
import android.view.MotionEvent;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import java.io.File;
import java.util.Arrays;

/** Platform scheduling only. The cooked package owns all game/UI content. */
public final class MainActivity extends Activity implements SurfaceHolder.Callback,
        Choreographer.FrameCallback, AudioManager.OnAudioFocusChangeListener {
    private SurfaceView view;
    private AudioManager audio;
    private AudioFocusRequest focusRequest;
    private File packageDirectory;
    private long owner;
    private long epoch;
    private final long[] gestures = new long[32];
    private boolean resumed, visible, focused, audioGranted, focusRequested, posted, failed;

    @Override public void onCreate(Bundle saved) {
        super.onCreate(saved);
        try {
            packageDirectory = PackageFiles.extract(getAssets(), getCodeCacheDir());
            audio = (AudioManager)getSystemService(AUDIO_SERVICE);
            if (audio == null) throw new IllegalStateException("AudioManager unavailable");
            focusRequest = new AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(new AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_GAME)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build())
                .setOnAudioFocusChangeListener(this).build();
            view = new SurfaceView(this);
            view.getHolder().addCallback(this);
            view.setOnTouchListener((v, event) -> handleTouch(event));
            setContentView(view);
        } catch (Exception | LinkageError error) { fail(error); }
    }
    private float density() {
        float density = getResources().getDisplayMetrics().density;
        if (!Float.isFinite(density) || density <= 0) throw new IllegalStateException("Invalid density");
        return density;
    }
    private void clearGestures() { epoch = 0; Arrays.fill(gestures, 0); }
    @Override public void surfaceCreated(SurfaceHolder holder) { /* size arrives in surfaceChanged */ }
    @Override public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        if (failed) return;
        try {
            clearGestures();
            if (width == 0 || height == 0) { surfaceDestroyed(holder); return; }
            float density = density();
            if (owner == 0) {
                owner = NativeBridge.open(getApplicationContext(), holder.getSurface(),
                    new File(packageDirectory, "package.json").getAbsolutePath(),
                    width, height, width / density, height / density);
                if (owner == 0) throw new IllegalStateException("Missing native owner");
            } else {
                NativeBridge.surface(owner, holder.getSurface(), width, height, width / density, height / density);
            }
            visible = width > 0 && height > 0;
            updateActive();
        } catch (Exception | LinkageError error) { fail(error); }
    }
    @Override public void surfaceDestroyed(SurfaceHolder holder) {
        visible = false; clearGestures();
        try {
            updateActive();
            if (owner != 0) NativeBridge.surface(owner, null, 0, 0, 1, 1);
        } catch (Exception | LinkageError error) { fail(error); }
    }
    @Override public void onResume() { super.onResume(); resumed = true; updateActive(); }
    @Override public void onPause() { resumed = false; updateActive(); super.onPause(); }
    @Override public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus); focused = hasFocus; updateActive();
    }
    @Override public void onAudioFocusChange(int change) {
        audioGranted = change == AudioManager.AUDIOFOCUS_GAIN;
        updateActive();
    }
    private void updateActive() {
        if (failed || audio == null) return;
        try {
            boolean wanted = resumed && focused && visible && owner != 0;
            if (!wanted && focusRequested) {
                audio.abandonAudioFocusRequest(focusRequest);
                focusRequested = false; audioGranted = false;
            } else if (wanted && !focusRequested) {
                focusRequested = true;
                audioGranted = audio.requestAudioFocus(focusRequest) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED;
            }
            boolean active = wanted && audioGranted;
            if (owner != 0) NativeBridge.active(owner, active);
            // Avoid invalidating a live gesture during repeated identical focus callbacks.
            if (!active) clearGestures();
            if (active && !posted) { posted = true; Choreographer.getInstance().postFrameCallback(this); }
            if (!active && posted) { Choreographer.getInstance().removeFrameCallback(this); posted = false; }
        } catch (Exception | LinkageError error) { fail(error); }
    }
    @Override public void doFrame(long frameTimeNanos) {
        posted = false;
        if (failed || !resumed || !focused || !visible || !audioGranted || owner == 0) return;
        try {
            long next = NativeBridge.frame(owner);
            if (epoch != next) Arrays.fill(gestures, 0);
            epoch = next;
            posted = true; Choreographer.getInstance().postFrameCallback(this);
        } catch (Exception | LinkageError error) { fail(error); }
    }
    private boolean handleTouch(MotionEvent event) {
        if (failed || epoch == 0 || owner == 0) return true;
        try {
            int action = event.getActionMasked();
            int count = event.getPointerCount();
            if (count > 32) throw new IllegalStateException("Pointer count limit");
            for (int i = 0; i < count; i++) {
                int phase;
                if (action == MotionEvent.ACTION_MOVE) phase = 1;
                else if (action == MotionEvent.ACTION_CANCEL) phase = 3;
                else if (i == event.getActionIndex() && (action == MotionEvent.ACTION_DOWN || action == MotionEvent.ACTION_POINTER_DOWN)) phase = 0;
                else if (i == event.getActionIndex() && (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_POINTER_UP)) phase = 2;
                else continue;
                int id = event.getPointerId(i);
                if (id < 0 || id >= 32) throw new IllegalStateException("Pointer ID limit");
                if (phase == 0) gestures[id] = epoch;
                long captured = gestures[id];
                // Suppress old releases; never relabel them with a replacement epoch.
                if (captured != 0 && captured == epoch) {
                    float density = density();
                    NativeBridge.touch(owner, captured, id, phase, event.getX(i)/density, event.getY(i)/density);
                }
                if (phase == 2 || phase == 3) gestures[id] = 0;
            }
        } catch (Exception | LinkageError error) { fail(error); }
        return true;
    }
    private void fail(Throwable error) {
        if (failed) return;
        failed = true;
        Log.e("IncantPlayer", "Native host stopped", error);
        stop();
        finish();
    }
    private void stop() {
        if (posted) { Choreographer.getInstance().removeFrameCallback(this); posted = false; }
        clearGestures();
        long old = owner; owner = 0;
        if (old != 0) {
            try { NativeBridge.close(old); } catch (Exception | LinkageError error) { Log.e("IncantPlayer", "Native close failed", error); }
        }
        if (focusRequested && audio != null) audio.abandonAudioFocusRequest(focusRequest);
        focusRequested = false; audioGranted = false;
    }
    @Override public void onDestroy() {
        stop();
        if (packageDirectory != null) PackageFiles.remove(packageDirectory);
        super.onDestroy();
    }
}
