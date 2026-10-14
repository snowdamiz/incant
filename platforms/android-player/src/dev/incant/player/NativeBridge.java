package dev.incant.player;

import android.content.Context;
import android.view.Surface;

/** Opaque IDs are owned by one UI thread; none is a native memory address. */
final class NativeBridge {
    static { System.loadLibrary("incant_game_player"); }
    private NativeBridge() {}
    static native long open(Context application, Surface surface, String packagePath,
                            int width, int height, float logicalWidth, float logicalHeight);
    static native void close(long owner);
    static native void active(long owner, boolean active);
    static native void surface(long owner, Surface surface, int width, int height,
                               float logicalWidth, float logicalHeight);
    static native long frame(long owner);
    static native void touch(long owner, long epoch, int pointer, int phase, float x, float y);
}
