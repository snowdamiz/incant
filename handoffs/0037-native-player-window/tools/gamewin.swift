// Review helper for handoff 0037 (Claude). It acts only on the exact game
// process whose executable resolves to $GAME_BINARY, and on Finder solely as the
// neutral foreground app for focus-loss cases. It never matches processes by
// name and never posts global input; window operations and captures target the
// game's own "Incant native game" window.
//
//   gamewin perms
//   gamewin windows PID
//   gamewin run PID OUTDIR "step; step; ..."   (or @PLANFILE)
//   gamewin analyze [--top PX] PNG...
//   gamewin diff A.png B.png
//
// Steps: cap NAME | burst PREFIX COUNT INTERVAL_MS | down KEY | up KEY |
// sleep MS | game-front | finder-front | finder-activate | size W H | pos X Y |
// minimize | restore | close | fullscreen 0|1 | drag X Y DX DY MOVES | state |
// require-focus | require-unfocus | audio-start NAME | audio-stop. Keys are D
// or F; key and mouse events go to PID only (CGEventPostToPid). A failed
// requirement stops the plan.
import AppKit
import ApplicationServices
import CoreGraphics
import Foundation
import ImageIO
import ScreenCaptureKit
import UniformTypeIdentifiers

let gameTitle = "Incant native game"
let runStart = Date().timeIntervalSince1970
var timeline: FileHandle?

struct Failure: Error, CustomStringConvertible { let description: String }

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data(("gamewin: " + message + "\n").utf8))
    exit(2)
}

func emit(_ fields: [String: Any]) {
    var record = fields
    let now = Date().timeIntervalSince1970
    record["t"] = (now - runStart) * 1000.0
    record["epoch"] = now
    let data = try! JSONSerialization.data(withJSONObject: record, options: [.sortedKeys])
    let line = String(decoding: data, as: UTF8.self) + "\n"
    FileHandle.standardOutput.write(Data(line.utf8))
    timeline?.write(Data(line.utf8))
}

// MARK: process identity

func executablePath(_ pid: pid_t) -> String? {
    var buffer = [CChar](repeating: 0, count: 4096)
    let length = proc_pidpath(pid, &buffer, UInt32(buffer.count))
    return length > 0 ? String(cString: buffer) : nil
}

func verifiedPid(_ text: String) -> pid_t {
    guard let pid = pid_t(text), pid > 1 else { fail("invalid pid \(text)") }
    guard let expected = ProcessInfo.processInfo.environment["GAME_BINARY"] else {
        fail("GAME_BINARY is not set")
    }
    let resolved = URL(fileURLWithPath: expected).resolvingSymlinksInPath().path
    guard let actual = executablePath(pid) else { fail("pid \(pid) is not running") }
    guard actual == resolved else { fail("pid \(pid) runs \(actual), not \(resolved)") }
    return pid
}

// MARK: window server

func windowRecords(_ pid: pid_t) -> [[String: Any]] {
    guard let list = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]]
    else { return [] }
    return list.filter { ($0[kCGWindowOwnerPID as String] as? Int) == Int(pid) }.map { info in
        let bounds = info[kCGWindowBounds as String] as? [String: Any] ?? [:]
        return [
            "id": info[kCGWindowNumber as String] as? Int ?? -1,
            "layer": info[kCGWindowLayer as String] as? Int ?? -1,
            "name": info[kCGWindowName as String] as? String ?? NSNull(),
            "on_screen": info[kCGWindowIsOnscreen as String] as? Bool ?? false,
            "alpha": info[kCGWindowAlpha as String] as? Double ?? -1,
            "bounds": bounds,
        ]
    }
}

func gameWindowId(_ pid: pid_t) -> CGWindowID {
    let candidates = windowRecords(pid).filter {
        ($0["layer"] as? Int) == 0 && ($0["name"] as? String) == gameTitle
    }
    guard candidates.count == 1, let id = candidates[0]["id"] as? Int else {
        fail("expected one layer-0 '\(gameTitle)' window for pid \(pid), found \(candidates.count)")
    }
    return CGWindowID(id)
}

// MARK: accessibility

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}

func gameAxWindow(_ pid: pid_t) -> AXUIElement {
    let app = AXUIElementCreateApplication(pid)
    let windows = attribute(app, kAXWindowsAttribute) as? [AXUIElement] ?? []
    let matches = windows.filter { attribute($0, kAXTitleAttribute) as? String == gameTitle }
    guard matches.count == 1 else {
        fail("expected one AX '\(gameTitle)' window for pid \(pid), found \(matches.count) of \(windows.count)")
    }
    return matches[0]
}

func axFrame(_ window: AXUIElement) -> [String: Any] {
    var origin = CGPoint.zero
    var size = CGSize.zero
    if let value = attribute(window, kAXPositionAttribute) {
        AXValueGetValue(value as! AXValue, .cgPoint, &origin)
    }
    if let value = attribute(window, kAXSizeAttribute) {
        AXValueGetValue(value as! AXValue, .cgSize, &size)
    }
    return [
        "x": origin.x, "y": origin.y, "w": size.width, "h": size.height,
        "minimized": attribute(window, kAXMinimizedAttribute) as? Bool ?? false,
        "main": attribute(window, kAXMainAttribute) as? Bool ?? false,
        "focused": attribute(window, kAXFocusedAttribute) as? Bool ?? false,
    ]
}

// The system-wide AX focused-application query returns kAXErrorCannotComplete
// on this host, so ask LaunchServices for the live front application instead.
func lsappinfo(_ arguments: [String]) -> String {
    let process = Process()
    let pipe = Pipe()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/lsappinfo")
    process.arguments = arguments
    process.standardOutput = pipe
    guard (try? process.run()) != nil else { return "" }
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    return String(decoding: data, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
}

func focusedApplicationPid() -> pid_t? {
    let front = lsappinfo(["front"])
    guard !front.isEmpty else { return nil }
    let info = lsappinfo(["info", "-only", "pid", front])
    guard let value = info.split(separator: "=").last else { return nil }
    return pid_t(value.trimmingCharacters(in: .whitespaces))
}

func check(_ error: AXError, _ what: String) {
    if error != .success { fail("\(what) failed with AXError \(error.rawValue)") }
}

func makeFrontmost(_ pid: pid_t) {
    let app = AXUIElementCreateApplication(pid)
    check(AXUIElementSetAttributeValue(app, kAXFrontmostAttribute as CFString, kCFBooleanTrue), "frontmost \(pid)")
}

func finderPid() -> pid_t {
    let finders = NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.finder")
    guard finders.count == 1 else { fail("expected one Finder, found \(finders.count)") }
    return finders[0].processIdentifier
}

// MARK: input

func keyCode(_ name: String) -> CGKeyCode {
    switch name {
    case "D": return 0x02
    case "F": return 0x03
    default: fail("unsupported key \(name)")
    }
}

func postKey(_ pid: pid_t, _ name: String, down: Bool) {
    let source = CGEventSource(stateID: .privateState)
    guard let event = CGEvent(keyboardEventSource: source, virtualKey: keyCode(name), keyDown: down)
    else { fail("cannot create key event") }
    event.postToPid(pid)
}

// MARK: capture

func shareableWindow(_ id: CGWindowID) async throws -> SCWindow {
    let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
    guard let window = content.windows.first(where: { $0.windowID == id }) else {
        throw Failure(description: "window \(id) is not shareable")
    }
    return window
}

func writePNG(_ image: CGImage, _ url: URL) throws {
    guard let destination = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)
    else { throw Failure(description: "cannot create \(url.path)") }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else { throw Failure(description: "cannot write \(url.path)") }
}

func capture(_ window: SCWindow, _ url: URL) async throws -> [String: Any] {
    let filter = SCContentFilter(desktopIndependentWindow: window)
    let scale = CGFloat(filter.pointPixelScale)
    let configuration = SCStreamConfiguration()
    configuration.width = Int((filter.contentRect.width * scale).rounded())
    configuration.height = Int((filter.contentRect.height * scale).rounded())
    configuration.showsCursor = false
    configuration.ignoreShadowsSingleWindow = true
    configuration.captureResolution = .best
    let requested = Date().timeIntervalSince1970
    let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: configuration)
    let returned = Date().timeIntervalSince1970
    try writePNG(image, url)
    return [
        "path": url.path, "px_w": image.width, "px_h": image.height, "scale": scale,
        "frame_pt": ["x": window.frame.minX, "y": window.frame.minY, "w": window.frame.width, "h": window.frame.height],
        "on_screen": window.isOnScreen, "capture_ms": (returned - requested) * 1000.0,
    ]
}

// MARK: app audio

// Captures only the game application's mixed output through ScreenCaptureKit
// and logs per-buffer channel RMS. This observes the system audio path for that
// process; it is neither a speaker measurement nor the offline PCM proof.
final class AudioTap: NSObject, SCStreamOutput, SCStreamDelegate {
    let handle: FileHandle
    let queue = DispatchQueue(label: "gamewin.audio")
    var stream: SCStream?
    var buffers = 0

    init(_ handle: FileHandle) { self.handle = handle }

    static func start(pid: pid_t, log: URL) async throws -> AudioTap {
        FileManager.default.createFile(atPath: log.path, contents: nil)
        let tap = AudioTap(try FileHandle(forWritingTo: log))
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
        guard let app = content.applications.first(where: { $0.processID == pid }) else {
            throw Failure(description: "game pid \(pid) is not a shareable application")
        }
        guard let display = content.displays.first else { throw Failure(description: "no display") }
        let filter = SCContentFilter(display: display, including: [app], exceptingWindows: [])
        let configuration = SCStreamConfiguration()
        configuration.capturesAudio = true
        configuration.excludesCurrentProcessAudio = true
        configuration.sampleRate = 48_000
        configuration.channelCount = 2
        configuration.width = 2
        configuration.height = 2
        configuration.minimumFrameInterval = CMTime(value: 1, timescale: 1)
        let stream = SCStream(filter: filter, configuration: configuration, delegate: tap)
        try stream.addStreamOutput(tap, type: .audio, sampleHandlerQueue: tap.queue)
        try await stream.startCapture()
        tap.stream = stream
        return tap
    }

    func stop() async throws {
        try await stream?.stopCapture()
        queue.sync { try? handle.close() }
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer buffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .audio else { return }
        let received = Date().timeIntervalSince1970
        let frames = CMSampleBufferGetNumSamples(buffer)
        var rms: [Double] = []
        try? buffer.withAudioBufferList { list, _ in
            for channel in list {
                guard let data = channel.mData else { continue }
                let count = Int(channel.mDataByteSize) / MemoryLayout<Float>.size
                let samples = data.bindMemory(to: Float.self, capacity: count)
                var sum = 0.0
                for i in 0..<count { sum += Double(samples[i]) * Double(samples[i]) }
                rms.append(count > 0 ? (sum / Double(count)).squareRoot() : 0)
            }
        }
        buffers += 1
        let record: [String: Any] = ["epoch": received, "t": (received - runStart) * 1000.0, "frames": frames, "rms": rms]
        if let data = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            handle.write(data + Data("\n".utf8))
        }
    }

    func stream(_ stream: SCStream, didStopWithError error: Error) {
        let record = "{\"error\":\"\(error.localizedDescription)\"}\n"
        handle.write(Data(record.utf8))
    }
}

var audio: AudioTap?

// MARK: plan runner

func steps(_ plan: String) throws -> [[String]] {
    let text = plan.hasPrefix("@") ? try String(contentsOfFile: String(plan.dropFirst()), encoding: .utf8) : plan
    return text.split(whereSeparator: { $0 == ";" || $0 == "\n" })
        .map { $0.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false).first.map(String.init) ?? "" }
        .map { $0.split(separator: " ").map(String.init) }
        .filter { !$0.isEmpty }
}

func state(_ pid: pid_t) -> [String: Any] {
    let focused = focusedApplicationPid()
    var fields: [String: Any] = ["focused_pid": focused.map { Int($0) } ?? NSNull()]
    fields["game_focused"] = (focused == pid)
    fields["cg"] = windowRecords(pid).filter { ($0["layer"] as? Int) == 0 }
    fields["ax"] = axFrame(gameAxWindow(pid))
    return fields
}

func run(_ pid: pid_t, _ outdir: URL, _ plan: String) async throws {
    try FileManager.default.createDirectory(at: outdir, withIntermediateDirectories: true)
    let log = outdir.appendingPathComponent("timeline.jsonl")
    if !FileManager.default.fileExists(atPath: log.path) {
        FileManager.default.createFile(atPath: log.path, contents: nil)
    }
    timeline = try FileHandle(forWritingTo: log)
    timeline?.seekToEndOfFile()
    let windowId = gameWindowId(pid)
    emit(["step": "begin", "pid": Int(pid), "window_id": Int(windowId), "plan": plan])
    for step in try steps(plan) {
        _ = verifiedPid(String(pid))
        let arguments = Array(step.dropFirst())
        switch step[0] {
        case "cap":
            let window = try await shareableWindow(windowId)
            var fields = try await capture(window, outdir.appendingPathComponent(arguments[0] + ".png"))
            fields["step"] = "cap"
            emit(fields)
        case "burst":
            let count = Int(arguments[1])!, interval = Double(arguments[2])! / 1000.0
            let window = try await shareableWindow(windowId)
            for index in 0..<count {
                let begin = Date().timeIntervalSince1970
                let name = String(format: "%@-%02d.png", arguments[0], index)
                var fields = try await capture(window, outdir.appendingPathComponent(name))
                fields["step"] = "burst"
                fields["index"] = index
                emit(fields)
                let remaining = interval - (Date().timeIntervalSince1970 - begin)
                if remaining > 0 { try await Task.sleep(nanoseconds: UInt64(remaining * 1e9)) }
            }
        case "down", "up":
            if focusedApplicationPid() != pid { emit(["step": "warn", "message": "game not focused before \(step[0])"]) }
            postKey(pid, arguments[0], down: step[0] == "down")
            emit(["step": step[0], "key": arguments[0]])
        case "sleep":
            try await Task.sleep(nanoseconds: UInt64(Double(arguments[0])! * 1e6))
            emit(["step": "sleep", "ms": Double(arguments[0])!])
        case "game-front":
            makeFrontmost(pid)
            check(AXUIElementPerformAction(gameAxWindow(pid), kAXRaiseAction as CFString), "raise")
            emit(["step": "game-front"])
        case "finder-front":
            makeFrontmost(finderPid())
            emit(["step": "finder-front"])
        case "finder-activate":
            // LaunchServices activation without a reopen event, so Finder opens no window.
            let finder = NSRunningApplication(processIdentifier: finderPid())
            let accepted = finder?.activate(options: []) ?? false
            emit(["step": "finder-activate", "accepted": accepted])
        case "size":
            var size = CGSize(width: Double(arguments[0])!, height: Double(arguments[1])!)
            let value = AXValueCreate(.cgSize, &size)!
            check(AXUIElementSetAttributeValue(gameAxWindow(pid), kAXSizeAttribute as CFString, value), "size")
            emit(["step": "size", "requested_pt": [size.width, size.height]])
        case "fullscreen":
            let flag = arguments[0] == "1" ? kCFBooleanTrue! : kCFBooleanFalse!
            check(AXUIElementSetAttributeValue(gameAxWindow(pid), "AXFullScreen" as CFString, flag), "fullscreen")
            emit(["step": "fullscreen", "value": arguments[0]])
        case "drag":
            // Left-button edge drag delivered to the game PID only; the real cursor never moves.
            let x = Double(arguments[0])!, y = Double(arguments[1])!
            let dx = Double(arguments[2])!, dy = Double(arguments[3])!, count = Int(arguments[4])!
            func post(_ type: CGEventType, _ point: CGPoint) {
                let source = CGEventSource(stateID: .privateState)
                guard let event = CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: point, mouseButton: .left)
                else { fail("cannot create mouse event") }
                event.setIntegerValueField(.mouseEventClickState, value: 1)
                event.setIntegerValueField(.mouseEventWindowUnderMousePointer, value: Int64(windowId))
                event.setIntegerValueField(.mouseEventWindowUnderMousePointerThatCanHandleThisEvent, value: Int64(windowId))
                event.postToPid(pid)
            }
            post(.leftMouseDown, CGPoint(x: x, y: y))
            for index in 1...count {
                try await Task.sleep(nanoseconds: 16_000_000)
                let fraction = Double(index) / Double(count)
                post(.leftMouseDragged, CGPoint(x: x + dx * fraction, y: y + dy * fraction))
            }
            try await Task.sleep(nanoseconds: 16_000_000)
            post(.leftMouseUp, CGPoint(x: x + dx, y: y + dy))
            emit(["step": "drag", "from": [x, y], "delta": [dx, dy], "moves": count])
        case "pos":
            var origin = CGPoint(x: Double(arguments[0])!, y: Double(arguments[1])!)
            let value = AXValueCreate(.cgPoint, &origin)!
            check(AXUIElementSetAttributeValue(gameAxWindow(pid), kAXPositionAttribute as CFString, value), "pos")
            emit(["step": "pos", "requested_pt": [origin.x, origin.y]])
        case "audio-start":
            audio = try await AudioTap.start(pid: pid, log: outdir.appendingPathComponent(arguments[0] + ".jsonl"))
            emit(["step": "audio-start", "path": arguments[0] + ".jsonl"])
        case "audio-stop":
            try await audio?.stop()
            emit(["step": "audio-stop", "buffers": audio?.buffers ?? 0])
            audio = nil
        case "minimize", "restore":
            let flag = step[0] == "minimize" ? kCFBooleanTrue! : kCFBooleanFalse!
            check(AXUIElementSetAttributeValue(gameAxWindow(pid), kAXMinimizedAttribute as CFString, flag), step[0])
            emit(["step": step[0]])
        case "close":
            guard let button = attribute(gameAxWindow(pid), kAXCloseButtonAttribute) else { fail("no close button") }
            check(AXUIElementPerformAction(button as! AXUIElement, kAXPressAction as CFString), "close")
            emit(["step": "close"])
            return
        case "state":
            var fields = state(pid)
            fields["step"] = "state"
            emit(fields)
        case "require-focus", "require-unfocus":
            let focused = focusedApplicationPid()
            let wanted = step[0] == "require-focus"
            emit(["step": step[0], "focused_pid": focused.map { Int($0) } ?? NSNull()])
            if (focused == pid) != wanted { fail("\(step[0]) failed: front pid \(focused.map { String($0) } ?? "none")") }
        default:
            fail("unknown step \(step[0])")
        }
    }
    emit(["step": "end"])
}

// MARK: analysis

func rgba(_ path: String) -> (width: Int, height: Int, pixels: [UInt8]) {
    guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
    else { fail("cannot read \(path)") }
    let width = image.width, height = image.height
    var pixels = [UInt8](repeating: 0, count: width * height * 4)
    let space = image.colorSpace ?? CGColorSpace(name: CGColorSpace.sRGB)!
    guard let context = CGContext(
        data: &pixels, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
        space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
    else { fail("cannot create context for \(path)") }
    context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
    return (width, height, pixels)
}

func difference(_ first: String, _ second: String) -> [String: Any] {
    let a = rgba(first), b = rgba(second)
    guard a.width == b.width, a.height == b.height else {
        return ["a": first, "b": second, "same_size": false]
    }
    var count = 0, maximum = 0
    var minX = Int.max, minY = Int.max, maxX = -1, maxY = -1
    for y in 0..<a.height {
        for x in 0..<a.width {
            let i = (y * a.width + x) * 4
            var delta = 0
            for c in 0..<4 { delta = max(delta, abs(Int(a.pixels[i + c]) - Int(b.pixels[i + c]))) }
            if delta == 0 { continue }
            count += 1
            maximum = max(maximum, delta)
            minX = min(minX, x); minY = min(minY, y); maxX = max(maxX, x); maxY = max(maxY, y)
        }
    }
    var result: [String: Any] = ["a": first, "b": second, "same_size": true, "differing_px": count, "max_delta": maximum]
    if count > 0 { result["bbox"] = [minX, minY, maxX, maxY] }
    return result
}

func analyze(_ path: String, top: Int) -> [String: Any] {
    let (width, height, pixels) = rgba(path)
    var histogram: [UInt32: Int] = [:]
    for y in top..<height {
        for x in 0..<width {
            let i = (y * width + x) * 4
            if pixels[i + 3] < 255 { continue }
            let key = UInt32(pixels[i]) << 16 | UInt32(pixels[i + 1]) << 8 | UInt32(pixels[i + 2])
            histogram[key, default: 0] += 1
        }
    }
    guard let background = histogram.max(by: { $0.value < $1.value }) else {
        return ["path": path, "w": width, "h": height, "opaque_content_px": 0]
    }
    let bg = [Int(background.key >> 16), Int(background.key >> 8 & 0xff), Int(background.key & 0xff)]
    var count = 0, sumX = 0.0, sumY = 0.0
    var minX = Int.max, minY = Int.max, maxX = -1, maxY = -1
    for y in top..<height {
        for x in 0..<width {
            let i = (y * width + x) * 4
            if pixels[i + 3] < 255 { continue }
            let delta = max(abs(Int(pixels[i]) - bg[0]), abs(Int(pixels[i + 1]) - bg[1]), abs(Int(pixels[i + 2]) - bg[2]))
            if delta <= 8 { continue }
            count += 1
            sumX += Double(x); sumY += Double(y)
            minX = min(minX, x); minY = min(minY, y); maxX = max(maxX, x); maxY = max(maxY, y)
        }
    }
    let opaque = histogram.values.reduce(0, +)
    var result: [String: Any] = [
        "path": path, "w": width, "h": height, "top": top, "opaque_content_px": opaque,
        "background_rgb": bg, "background_px": background.value, "distinct_colors": histogram.count,
        "foreground_px": count,
    ]
    if count > 0 {
        result["centroid"] = [sumX / Double(count), sumY / Double(count)]
        result["bbox"] = [minX, minY, maxX, maxY]
    }
    return result
}

// MARK: entry

@main
struct GameWin {
    static func main() async {
        let arguments = Array(CommandLine.arguments.dropFirst())
        guard let command = arguments.first else { fail("missing command") }
        switch command {
        case "perms":
            emit([
                "accessibility_trusted": AXIsProcessTrusted(),
                "screen_capture": CGPreflightScreenCaptureAccess(),
                "post_event": CGPreflightPostEventAccess(),
                "focused_pid": focusedApplicationPid().map { Int($0) } ?? NSNull(),
            ])
        case "windows":
            let pid = verifiedPid(arguments[1])
            emit(["pid": Int(pid), "windows": windowRecords(pid), "focused_pid": focusedApplicationPid().map { Int($0) } ?? NSNull()])
        case "run":
            let pid = verifiedPid(arguments[1])
            do {
                try await run(pid, URL(fileURLWithPath: arguments[2]), arguments[3])
            } catch {
                fail("\(error)")
            }
        case "analyze":
            var top = 0
            var paths = Array(arguments.dropFirst())
            if paths.first == "--top" { top = Int(paths[1])!; paths = Array(paths.dropFirst(2)) }
            for path in paths { emit(analyze(path, top: top)) }
        case "diff":
            emit(difference(arguments[1], arguments[2]))
        default:
            fail("unknown command \(command)")
        }
    }
}
