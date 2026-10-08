package dev.incant.smoke;
import android.app.Activity;
import android.os.Bundle;
import android.util.Log;
public final class MainActivity extends Activity {
    static { System.loadLibrary("incant_platform_smoke"); }
    public static native int runSmoke();
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        int result = runSmoke();
        Log.i("IncantSmoke", result == 0 ? "INCANT_SMOKE_PASS" : "INCANT_SMOKE_FAIL");
    }
}
