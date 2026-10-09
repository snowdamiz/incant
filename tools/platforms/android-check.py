#!/usr/bin/env python3
"""Run the real APK in a disposable CI emulator; never target an attached device."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import time


def instrumentation_passed(output):
    codes = re.findall(r"^INSTRUMENTATION_CODE: (-?\d+)\s*$", output, re.MULTILINE)
    return (
        codes == ["-1"]
        and "INCANT_SMOKE_PASS" in output
        and not any(word in output for word in ("INCANT_SMOKE_FAIL", "INSTRUMENTATION_FAILED", "Exception"))
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--avd", required=True, help="Disposable AVD created for this job; its data is wiped")
    parser.add_argument("--sdk", default=os.environ.get("ANDROID_HOME"))
    parser.add_argument("--port", type=int, default=5580)
    parser.add_argument("--apk", type=Path, default=Path("artifacts/android/IncantSmoke-debug.apk"))
    args = parser.parse_args()
    if not args.sdk or not args.apk.is_file():
        parser.error("An installed SDK and built APK are required")
    if args.port % 2 or not 5554 <= args.port <= 5682:
        parser.error("Use an even emulator port between 5554 and 5682")
    sdk = Path(args.sdk)
    adb = str(sdk / "platform-tools/adb")
    emulator = str(sdk / "emulator/emulator")
    serial = f"emulator-{args.port}"
    # Refuse an occupied console/ADB port before starting our own emulator.
    for port in (args.port, args.port + 1):
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", port))
    output = args.apk.resolve().parent
    report_path = output / "execution.json"
    report_path.unlink(missing_ok=True)

    def command(*parts, timeout=30, check=True):
        result = subprocess.run(
            [adb, "-s", serial, *parts], text=True, capture_output=True,
            timeout=timeout, check=check,
        )
        return result.stdout.strip()

    with (output / "emulator.log").open("w") as log:
        process = subprocess.Popen([
            emulator, "-avd", args.avd, "-port", str(args.port),
            "-no-window", "-no-audio", "-no-boot-anim", "-no-snapshot",
            "-wipe-data", "-gpu", "swiftshader_indirect", "-accel", "on",
            "-camera-back", "none", "-camera-front", "none",
        ], stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 240
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError("Emulator exited before boot; inspect emulator.log")
                try:
                    if command("shell", "getprop", "sys.boot_completed", timeout=10, check=False) == "1":
                        break
                except subprocess.TimeoutExpired:
                    pass
                time.sleep(2)
            else:
                raise RuntimeError("Emulator boot deadline exceeded")

            command("install", "-r", str(args.apk.resolve()), timeout=90)
            command("logcat", "-c")
            launch = command("shell", "am", "start", "-W", "-n", "dev.incant.smoke/.MainActivity")
            (output / "activity-launch.txt").write_text(launch + "\n")
            deadline = time.monotonic() + 30
            activity_log = ""
            while time.monotonic() < deadline:
                activity_log = command("logcat", "-d", "-s", "IncantSmoke:I", "*:S")
                if "INCANT_SMOKE_PASS" in activity_log or "INCANT_SMOKE_FAIL" in activity_log:
                    break
                time.sleep(1)
            (output / "activity.log").write_text(activity_log + "\n")
            if "INCANT_SMOKE_PASS" not in activity_log or "INCANT_SMOKE_FAIL" in activity_log:
                raise RuntimeError("Activity did not complete the native probe")
            command("shell", "am", "force-stop", "dev.incant.smoke")
            result = command("shell", "am", "instrument", "-w", "-r",
                             "dev.incant.smoke/.SmokeInstrumentation", timeout=60)
            (output / "instrumentation.txt").write_text(result + "\n")
            # adb may return exit 0 even when instrumentation fails.
            if not instrumentation_passed(result):
                raise RuntimeError("Instrumentation did not report a successful native probe")
            report = {
                "platform": "android", "execution": "hosted-emulator",
                "abi": command("shell", "getprop", "ro.product.cpu.abi"),
                "api_level": command("shell", "getprop", "ro.build.version.sdk"),
                "apk_sha256": hashlib.sha256(args.apk.read_bytes()).hexdigest(),
                "activity_passed": True, "instrumentation_passed": True,
                "native_document_and_120_tick_probe_passed": True,
                "physical_device": False, "visual_review": False,
            }
            report_path.write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps(report))
        finally:
            if process.poll() is None:
                try:
                    command("emu", "kill", timeout=10, check=False)
                except subprocess.TimeoutExpired:
                    pass
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)


if __name__ == "__main__":
    main()
