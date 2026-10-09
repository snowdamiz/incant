package dev.incant.smoke;
import android.app.Activity;
import android.app.Instrumentation;
import android.os.Bundle;
public final class SmokeInstrumentation extends Instrumentation {
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        int result = MainActivity.runSmoke();
        Bundle report = new Bundle();
        report.putString("stream", result == 0 ? "INCANT_SMOKE_PASS" : "INCANT_SMOKE_FAIL");
        finish(result == 0 ? Activity.RESULT_OK : Activity.RESULT_CANCELED, report);
    }
}
