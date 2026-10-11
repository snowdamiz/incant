// PID-scoped native review helper for handoff 0036.
//
// Every command that touches the app first proves the target's identity: the
// PID's kernel executable path, NSRunningApplication's executable URL and its
// bundle identifier must all match the prepared review bundle. Input commands
// then require that exact PID to be frontmost (by NSWorkspace and by the
// system-wide accessibility focus) immediately before each event, and post
// keyboard events to that PID only (CGEvent.postToPid), never to the global
// event stream. Captures use `screencapture -l <window id>` for the review
// window alone. Each command appends one JSON line to $NATIVE_LOG.
//
// Build: swiftc -O handoffs/0036-editor-final-native/tools/native.swift -o <out>
import AppKit
import ApplicationServices
import CryptoKit
import Darwin
import Foundation

let expectedBundle = "dev.incant.review.28e5491"

// MARK: - Output and logging

func emit(_ object: Any) {
    let data = try! JSONSerialization.data(withJSONObject: object, options: [.sortedKeys, .withoutEscapingSlashes])
    print(String(data: data, encoding: .utf8)!)
}

func log(_ entry: [String: Any]) {
    guard let path = ProcessInfo.processInfo.environment["NATIVE_LOG"] else { return }
    var line = entry
    line["time"] = ISO8601DateFormatter.string(from: Date(), timeZone: .current, formatOptions: [.withInternetDateTime, .withFractionalSeconds])
    let data = try! JSONSerialization.data(withJSONObject: line, options: [.sortedKeys, .withoutEscapingSlashes])
    if let handle = FileHandle(forWritingAtPath: path) {
        handle.seekToEndOfFile()
        handle.write(data + Data("\n".utf8))
        handle.closeFile()
    } else {
        FileManager.default.createFile(atPath: path, contents: data + Data("\n".utf8))
    }
}

func die(_ message: String, _ extra: [String: Any] = [:]) -> Never {
    var entry = extra
    entry["error"] = message
    entry["argv"] = Array(CommandLine.arguments.dropFirst())
    log(entry)
    FileHandle.standardError.write(Data("native: \(message)\n".utf8))
    exit(2)
}

// MARK: - Arguments

var args = Array(CommandLine.arguments.dropFirst())
guard !args.isEmpty else { die("usage: native <command> [--pid P --exe PATH] ...") }
let command = args.removeFirst()

func option(_ name: String) -> String? {
    guard let i = args.firstIndex(of: name), i + 1 < args.count else { return nil }
    let value = args[i + 1]
    args.removeSubrange(i...(i + 1))
    return value
}
func flag(_ name: String) -> Bool {
    guard let i = args.firstIndex(of: name) else { return false }
    args.remove(at: i)
    return true
}

// MARK: - Identity and frontmost assertions

func kernelPath(_ pid: pid_t) -> String? {
    var buffer = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
    let n = proc_pidpath(pid, &buffer, UInt32(buffer.count))
    return n > 0 ? String(cString: buffer) : nil
}

struct Target {
    let pid: pid_t
    let exe: String
    let app: NSRunningApplication
    let element: AXUIElement
}

func identity(_ pid: pid_t, _ exe: String) -> Target {
    guard let app = NSRunningApplication(processIdentifier: pid), !app.isTerminated else {
        die("no running application for pid \(pid)", ["pid": pid])
    }
    guard let path = kernelPath(pid), path == exe else {
        die("kernel executable path mismatch", ["pid": pid, "kernel_path": kernelPath(pid) ?? "nil"])
    }
    guard app.executableURL?.path == exe else {
        die("NSRunningApplication executable mismatch", ["pid": pid, "url": app.executableURL?.path ?? "nil"])
    }
    // Launched by exec under sandbox-exec, LaunchServices registers the bundle path but no
    // bundle ID (the ad-hoc linker signature does not bind Info.plist). Require the exact
    // registered bundle, whose on-disk Info.plist names the review ID, and refuse any other ID.
    guard let bundleURL = app.bundleURL, exe.hasPrefix(bundleURL.path + "/Contents/MacOS/"),
          Bundle(url: bundleURL)?.bundleIdentifier == expectedBundle else {
        die("bundle mismatch", ["pid": pid, "bundle_path": app.bundleURL?.path ?? "nil"])
    }
    guard app.bundleIdentifier == nil || app.bundleIdentifier == expectedBundle else {
        die("bundle identifier mismatch", ["pid": pid, "bundle": app.bundleIdentifier ?? "nil"])
    }
    let element = AXUIElementCreateApplication(pid)
    AXUIElementSetMessagingTimeout(element, 3)
    return Target(pid: pid, exe: exe, app: app, element: element)
}

func target() -> Target {
    guard let pidText = option("--pid"), let pid = pid_t(pidText), let exe = option("--exe") else {
        die("--pid and --exe are required")
    }
    return identity(pid, exe)
}

/// Three independent views of "frontmost": LaunchServices via NSWorkspace, the window
/// server's front on-screen normal window, and the app's own AXFrontmost attribute.
/// (The system-wide AX element returns kAXErrorCannotComplete in this session.)
struct Front {
    let workspace: pid_t?
    let window: pid_t?
    let axFrontmost: Bool?
    func isReview(_ pid: pid_t) -> Bool { workspace == pid && window == pid && axFrontmost == true }
    var log: [String: Any] {
        ["front_workspace": workspace.map { Int($0) } ?? -1, "front_window_owner": window.map { Int($0) } ?? -1,
         "ax_frontmost": axFrontmost.map { $0 ? "true" : "false" } ?? "nil"]
    }
}

func frontWindowOwner() -> pid_t? {
    let list = (CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]]) ?? []
    return list.first { ($0[kCGWindowLayer as String] as? Int) == 0 }.flatMap { $0[kCGWindowOwnerPID as String] as? Int32 }
}

func frontmost(_ pid: pid_t? = nil) -> Front {
    var ax: Bool?
    if let pid {
        let element = AXUIElementCreateApplication(pid)
        var value: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXFrontmostAttribute as CFString, &value) == .success { ax = value as? Bool }
    }
    return Front(workspace: NSWorkspace.shared.frontmostApplication?.processIdentifier, window: frontWindowOwner(), axFrontmost: ax)
}

/// Asserts the review PID is frontmost by all three sources; aborts before any input otherwise.
@discardableResult
func assertFrontmost(_ t: Target, _ action: String) -> [String: Any] {
    let front = frontmost(t.pid)
    var entry: [String: Any] = [
        "action": action, "pid": t.pid, "exe": t.exe, "bundle_path": t.app.bundleURL?.path ?? "nil",
        "bundle_id_on_disk": t.app.bundleURL.flatMap { Bundle(url: $0)?.bundleIdentifier } ?? "nil",
    ]
    entry.merge(front.log) { a, _ in a }
    guard front.isReview(t.pid) else {
        die("review PID is not frontmost; input refused", entry)
    }
    return entry
}

// MARK: - Accessibility

func attribute(_ el: AXUIElement, _ name: String) -> AnyObject? {
    var value: AnyObject?
    return AXUIElementCopyAttributeValue(el, name as CFString, &value) == .success ? value : nil
}

func text(_ el: AXUIElement, _ name: String) -> String? {
    guard let value = attribute(el, name) else { return nil }
    if let s = value as? String { return s }
    if let n = value as? NSNumber { return n.stringValue }
    if let a = value as? NSAttributedString { return a.string }
    return nil
}

func children(_ el: AXUIElement) -> [AXUIElement] {
    (attribute(el, kAXChildrenAttribute) as? [AXUIElement]) ?? []
}

func frame(_ el: AXUIElement) -> CGRect? {
    guard let p = attribute(el, kAXPositionAttribute), let s = attribute(el, kAXSizeAttribute),
          CFGetTypeID(p) == AXValueGetTypeID(), CFGetTypeID(s) == AXValueGetTypeID() else { return nil }
    var point = CGPoint.zero
    var size = CGSize.zero
    AXValueGetValue(p as! AXValue, .cgPoint, &point)
    AXValueGetValue(s as! AXValue, .cgSize, &size)
    return CGRect(origin: point, size: size)
}

func classes(_ el: AXUIElement) -> [String] {
    (attribute(el, "AXDOMClassList") as? [String]) ?? []
}

func describe(_ el: AXUIElement, depth: Int? = nil) -> [String: Any] {
    var d: [String: Any] = [:]
    for (key, name) in [("role", kAXRoleAttribute), ("subrole", kAXSubroleAttribute), ("title", kAXTitleAttribute),
                        ("desc", kAXDescriptionAttribute), ("help", kAXHelpAttribute), ("domid", "AXDOMIdentifier"),
                        ("placeholder", "AXPlaceholderValue"), ("roledesc", kAXRoleDescriptionAttribute)] {
        if let v = text(el, name), !v.isEmpty { d[key] = v }
    }
    if let v = text(el, kAXValueAttribute), !v.isEmpty { d["value"] = v.count > 300 ? String(v.prefix(300)) + "…" : v }
    let cls = classes(el)
    if !cls.isEmpty { d["class"] = cls.joined(separator: " ") }
    if (attribute(el, kAXFocusedAttribute) as? Bool) == true { d["focused"] = true }
    if let enabled = attribute(el, kAXEnabledAttribute) as? Bool, !enabled { d["disabled"] = true }
    if let selected = attribute(el, kAXSelectedAttribute) as? Bool, selected { d["selected"] = true }
    if let expanded = attribute(el, kAXExpandedAttribute) as? Bool { d["expanded"] = expanded }
    if let f = frame(el) { d["frame"] = [f.origin.x, f.origin.y, f.size.width, f.size.height].map { Double($0) } }
    if let depth { d["depth"] = depth }
    return d
}

struct Match {
    var role: String?, name: String?, nameContains: String?, domid: String?, cls: String?, valueContains: String?, index = 0
    init() {
        role = option("--role"); name = option("--name"); nameContains = option("--name~")
        domid = option("--domid"); cls = option("--class"); valueContains = option("--value~")
        index = Int(option("--index") ?? "0") ?? 0
    }
    var isEmpty: Bool { role == nil && name == nil && nameContains == nil && domid == nil && cls == nil && valueContains == nil }
    func matches(_ el: AXUIElement) -> Bool {
        if let role, text(el, kAXRoleAttribute) != role { return false }
        let names = [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute].compactMap { text(el, $0) }
        if let name, !names.contains(name) { return false }
        if let nameContains, !names.contains(where: { $0.contains(nameContains) }) { return false }
        if let domid, text(el, "AXDOMIdentifier") != domid { return false }
        if let cls, !classes(el).contains(cls) { return false }
        if let valueContains, !(text(el, kAXValueAttribute) ?? "").contains(valueContains) { return false }
        return true
    }
    var summary: [String: Any] {
        var d: [String: Any] = ["index": index]
        if let role { d["role"] = role }; if let name { d["name"] = name }; if let nameContains { d["name~"] = nameContains }
        if let domid { d["domid"] = domid }; if let cls { d["class"] = cls }; if let valueContains { d["value~"] = valueContains }
        return d
    }
}

/// Breadth-first search below `root`, bounded so a large web tree cannot stall the helper.
func findAll(_ root: AXUIElement, _ match: Match, limit: Int = 40000) -> [AXUIElement] {
    var queue = [root]
    var found: [AXUIElement] = []
    var seen = 0
    while !queue.isEmpty && seen < limit {
        let el = queue.removeFirst()
        seen += 1
        if match.matches(el) { found.append(el) }
        queue.append(contentsOf: children(el))
    }
    return found
}

func mainWindow(_ t: Target) -> AXUIElement {
    if let w = attribute(t.element, kAXMainWindowAttribute), CFGetTypeID(w) == AXUIElementGetTypeID() { return w as! AXUIElement }
    if let ws = attribute(t.element, kAXWindowsAttribute) as? [AXUIElement], let w = ws.first { return w }
    die("review app has no accessibility window", ["pid": t.pid])
}

func root(_ t: Target) -> AXUIElement {
    let byId = option("--within-domid")
    let byClass = option("--within-class")
    if byId != nil || byClass != nil {
        var m = Match(); m.domid = byId; m.cls = byClass
        guard let el = findAll(mainWindow(t), m).first else { die("no scope element", ["domid": byId ?? "", "class": byClass ?? ""]) }
        return el
    }
    return mainWindow(t)
}

func one(_ t: Target, _ match: Match) -> AXUIElement {
    guard !match.isEmpty else { die("a match criterion is required") }
    let all = findAll(root(t), match)
    guard match.index < all.count else { die("no matching element", ["match": match.summary, "count": all.count]) }
    return all[match.index]
}

func focusedElement(_ t: Target) -> AXUIElement? {
    guard let v = attribute(t.element, kAXFocusedUIElementAttribute), CFGetTypeID(v) == AXUIElementGetTypeID() else { return nil }
    return (v as! AXUIElement)
}

func ancestry(_ el: AXUIElement) -> [[String: Any]] {
    var chain: [[String: Any]] = []
    var current: AXUIElement? = el
    while let c = current, chain.count < 14 {
        var d = describe(c)
        d.removeValue(forKey: "value")
        chain.append(d)
        if text(c, kAXRoleAttribute) == kAXWindowRole { break }
        if let p = attribute(c, kAXParentAttribute), CFGetTypeID(p) == AXUIElementGetTypeID() { current = (p as! AXUIElement) } else { current = nil }
    }
    return chain
}

// MARK: - Keyboard

let keyCodes: [String: CGKeyCode] = [
    "a": 0x00, "s": 0x01, "d": 0x02, "f": 0x03, "h": 0x04, "g": 0x05, "z": 0x06, "x": 0x07, "c": 0x08, "v": 0x09,
    "b": 0x0B, "q": 0x0C, "w": 0x0D, "e": 0x0E, "r": 0x0F, "y": 0x10, "t": 0x11, "1": 0x12, "2": 0x13, "3": 0x14,
    "4": 0x15, "6": 0x16, "5": 0x17, "=": 0x18, "9": 0x19, "7": 0x1A, "-": 0x1B, "8": 0x1C, "0": 0x1D, "]": 0x1E,
    "o": 0x1F, "u": 0x20, "[": 0x21, "i": 0x22, "p": 0x23, "l": 0x25, "j": 0x26, "'": 0x27, "k": 0x28, ";": 0x29,
    "\\": 0x2A, ",": 0x2B, "/": 0x2C, "n": 0x2D, "m": 0x2E, ".": 0x2F, "`": 0x32, " ": 0x31,
]
let shifted: [Character: String] = ["?": "/", "_": "-", ":": ";", "\"": "'", "<": ",", ">": ".", "+": "=", "!": "1", "@": "2", "#": "3", "*": "8", "(": "9", ")": "0"]
let specialKeys: [String: (CGKeyCode, Bool)] = [  // (code, function-key flag)
    "return": (0x24, false), "enter": (0x24, false), "tab": (0x30, false), "space": (0x31, false),
    "backspace": (0x33, false), "escape": (0x35, false), "esc": (0x35, false),
    "f1": (0x7A, true), "f2": (0x78, true), "f3": (0x63, true), "f4": (0x76, true), "f5": (0x60, true), "f6": (0x61, true),
    "f7": (0x62, true), "f8": (0x64, true), "f9": (0x65, true), "f10": (0x6D, true), "f11": (0x67, true), "f12": (0x6F, true),
    "home": (0x73, true), "pageup": (0x74, true), "forwarddelete": (0x75, true), "end": (0x77, true), "pagedown": (0x79, true),
    "left": (0x7B, true), "right": (0x7C, true), "down": (0x7D, true), "up": (0x7E, true),
]

func post(_ t: Target, code: CGKeyCode, flags: CGEventFlags, characters: String?) {
    let source = CGEventSource(stateID: .privateState)
    for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down) else { die("cannot create key event") }
        event.flags = flags
        if let characters {
            let units = Array(characters.utf16)
            event.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
        }
        event.postToPid(t.pid)
        usleep(down ? 25_000 : 45_000)
    }
}

/// A combination such as "shift+f6", "alt+cmd+3", "cmd+/", "?" or "escape".
func sendCombo(_ t: Target, _ spec: String) {
    var flags: CGEventFlags = []
    var parts = spec.split(separator: "+", omittingEmptySubsequences: false).map(String.init)
    if spec.hasSuffix("++") || spec == "+" { parts = Array(parts.dropLast(2)) + ["+"] }
    let key = parts.removeLast()
    for m in parts {
        switch m.lowercased() {
        case "cmd", "command": flags.insert(.maskCommand)
        case "shift": flags.insert(.maskShift)
        case "alt", "option", "opt": flags.insert(.maskAlternate)
        case "ctrl", "control": flags.insert(.maskControl)
        default: die("unknown modifier \(m)")
        }
    }
    if let (code, function) = specialKeys[key.lowercased()] {
        if function { flags.insert(.maskSecondaryFn) }
        post(t, code: code, flags: flags, characters: nil)
        return
    }
    guard key.count == 1, let ch = key.first else { die("unknown key \(key)") }
    if let base = shifted[ch], let code = keyCodes[base] {
        flags.insert(.maskShift)
        post(t, code: code, flags: flags, characters: flags.contains(.maskCommand) ? base : String(ch))
    } else if let code = keyCodes[String(ch).lowercased()] {
        if ch.isUppercase { flags.insert(.maskShift) }
        post(t, code: code, flags: flags, characters: flags.contains(.maskCommand) ? String(ch).lowercased() : String(ch))
    } else {
        die("no key code for \(key)")
    }
}

// MARK: - Windows and capture

func windows(_ pid: pid_t) -> [[String: Any]] {
    let list = (CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]]) ?? []
    return list.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid }.map { w in
        let b = w[kCGWindowBounds as String] as? [String: Any] ?? [:]
        return [
            "id": w[kCGWindowNumber as String] as? Int ?? -1,
            "layer": w[kCGWindowLayer as String] as? Int ?? -1,
            "onscreen": w[kCGWindowIsOnscreen as String] as? Bool ?? false,
            "alpha": w[kCGWindowAlpha as String] as? Double ?? 0,
            "bounds": ["x": b["X"] ?? 0, "y": b["Y"] ?? 0, "w": b["Width"] ?? 0, "h": b["Height"] ?? 0],
        ]
    }
}

func reviewWindow(_ pid: pid_t) -> [String: Any] {
    let candidates = windows(pid).filter { ($0["layer"] as? Int) == 0 && ($0["onscreen"] as? Bool) == true }
    func area(_ w: [String: Any]) -> Double {
        let b = w["bounds"] as! [String: Any]
        return ((b["w"] as? Double) ?? 0) * ((b["h"] as? Double) ?? 0)
    }
    guard let best = candidates.max(by: { area($0) < area($1) }) else { die("no on-screen layer-0 window for pid \(pid)") }
    return best
}

func sha256(_ path: String) -> String {
    let data = FileManager.default.contents(atPath: path) ?? Data()
    return SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
}

func pngSize(_ path: String) -> [Int] {
    guard let image = NSImage(contentsOfFile: path), let rep = image.representations.first else { return [0, 0] }
    return [rep.pixelsWide, rep.pixelsHigh]
}

// MARK: - Commands

switch command {
case "trusted":
    emit(["accessibility": AXIsProcessTrusted(), "screen_capture": CGPreflightScreenCaptureAccess()])

case "front":
    let f = frontmost()
    let app = f.workspace.flatMap { NSRunningApplication(processIdentifier: $0) }
    // Report only the frontmost PIDs and whether it is the review bundle; no other app details.
    var out = f.log
    out["front_is_review"] = app?.bundleURL.flatMap { Bundle(url: $0)?.bundleIdentifier } == expectedBundle
    emit(out)

case "identify":
    let t = target()
    let window = attribute(t.element, kAXMainWindowAttribute).flatMap { CFGetTypeID($0) == AXUIElementGetTypeID() ? ($0 as! AXUIElement) : nil }
    var out: [String: Any] = [
        "pid": t.pid, "kernel_path": kernelPath(t.pid) ?? "nil", "executable": t.app.executableURL?.path ?? "nil",
        "ls_bundle_identifier": t.app.bundleIdentifier ?? "nil", "bundle_path": t.app.bundleURL?.path ?? "nil",
        "bundle_id_on_disk": t.app.bundleURL.flatMap { Bundle(url: $0)?.bundleIdentifier } ?? "nil",
        "active": t.app.isActive, "windows": windows(t.pid),
    ]
    if let window { out["ax_window"] = describe(window) }
    if let launched = t.app.launchDate { out["launch_date"] = ISO8601DateFormatter().string(from: launched) }
    log(["action": "identify", "pid": t.pid, "exe": t.exe])
    emit(out)

case "raise":
    let t = target()
    AXUIElementSetAttributeValue(t.element, kAXFrontmostAttribute as CFString, kCFBooleanTrue)
    let w = mainWindow(t)
    AXUIElementPerformAction(w, kAXRaiseAction as CFString)
    var ok = false
    for _ in 0..<40 {
        if frontmost(t.pid).isReview(t.pid) { ok = true; break }
        usleep(50_000)
    }
    var entry: [String: Any] = ["action": "raise", "pid": t.pid, "exe": t.exe, "frontmost": ok]
    entry.merge(frontmost(t.pid).log) { a, _ in a }
    log(entry)
    emit(entry)
    if !ok { exit(3) }

case "key":
    let t = target()
    let combos = args
    guard !combos.isEmpty else { die("key needs at least one combination") }
    for combo in combos {
        var entry = assertFrontmost(t, "key")
        sendCombo(t, combo)
        entry["combo"] = combo
        log(entry)
    }
    emit(["sent": combos])

case "type":
    let t = target()
    guard let value = args.first else { die("type needs text") }
    var entry = assertFrontmost(t, "type")
    for ch in value {
        assertFrontmost(t, "type-char")
        sendCombo(t, String(ch))
    }
    entry["text"] = value
    log(entry)
    emit(["typed": value])

case "press", "focus", "find", "dump", "texts":
    let t = target()
    let match = Match()
    switch command {
    case "find":
        let all = findAll(root(t), match)
        emit(["count": all.count, "matches": all.prefix(60).map { describe($0) }])
    case "dump":
        let maxDepth = Int(option("--depth") ?? "40") ?? 40
        let base: AXUIElement = match.isEmpty ? root(t) : one(t, match)
        var lines: [[String: Any]] = []
        func walk(_ el: AXUIElement, _ depth: Int) {
            guard depth <= maxDepth, lines.count < 6000 else { return }
            lines.append(describe(el, depth: depth))
            for c in children(el) { walk(c, depth + 1) }
        }
        walk(base, 0)
        emit(["count": lines.count, "nodes": lines])
    case "texts":
        // Visible text under a root, in tree order: what a reader of that region gets.
        let base: AXUIElement = match.isEmpty ? root(t) : one(t, match)
        // Accessible text in tree order. WebKit flattens visually hidden spans and reports
        // unclipped frames, so this includes screen-reader-only text; pixels decide visibility.
        var out: [String] = []
        func walk(_ el: AXUIElement) {
            guard out.count < 4000 else { return }
            if text(el, kAXRoleAttribute) == kAXStaticTextRole, let v = text(el, kAXValueAttribute),
               !v.trimmingCharacters(in: .whitespaces).isEmpty { out.append(v) }
            for c in children(el) { walk(c) }
        }
        walk(base)
        emit(["count": out.count, "accessible_texts": out])
    default:
        let el = one(t, match)
        var entry = assertFrontmost(t, command)
        entry["match"] = match.summary
        entry["element"] = describe(el)
        let result: AXError
        if command == "press" {
            result = AXUIElementPerformAction(el, kAXPressAction as CFString)
        } else {
            result = AXUIElementSetAttributeValue(el, kAXFocusedAttribute as CFString, kCFBooleanTrue)
        }
        entry["ax_result"] = Int(result.rawValue)
        log(entry)
        emit(["result": Int(result.rawValue), "element": describe(el)])
        if result != .success { exit(4) }
    }

case "focused":
    let t = target()
    let match = Match()
    let timeout = Double(option("--timeout") ?? "0") ?? 0
    let deadline = Date().addingTimeInterval(timeout)
    var el = focusedElement(t)
    while !match.isEmpty, Date() < deadline, !(el.map { match.matches($0) } ?? false) {
        usleep(100_000)
        el = focusedElement(t)
    }
    guard let el else { emit(["focused": NSNull()]); exit(match.isEmpty ? 0 : 5) }
    let ok = match.isEmpty || match.matches(el)
    emit(["focused": describe(el), "ancestry": ancestry(el), "matched": ok])
    if !ok { exit(5) }

case "resize":
    let t = target()
    guard args.count >= 2, let w = Double(args[0]), let h = Double(args[1]) else { die("resize W H [X Y]") }
    let window = mainWindow(t)
    var entry = assertFrontmost(t, "resize")
    if args.count >= 4, let x = Double(args[2]), let y = Double(args[3]) {
        var p = CGPoint(x: x, y: y)
        AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, AXValueCreate(.cgPoint, &p)!)
    }
    var s = CGSize(width: w, height: h)
    let r = AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, AXValueCreate(.cgSize, &s)!)
    usleep(400_000)
    entry["requested"] = [w, h]
    entry["result"] = Int(r.rawValue)
    entry["frame"] = frame(window).map { [$0.origin.x, $0.origin.y, $0.size.width, $0.size.height].map { Double($0) } } ?? []
    log(entry)
    emit(entry)

case "menu":
    // Presses a native menu bar item of the review app: menu "Edit" "Undo".
    let t = target()
    guard args.count >= 2 else { die("menu <menu title> <item title>") }
    guard let bar = attribute(t.element, kAXMenuBarAttribute), CFGetTypeID(bar) == AXUIElementGetTypeID() else { die("no menu bar") }
    let barItems = children(bar as! AXUIElement)
    guard let top = barItems.first(where: { text($0, kAXTitleAttribute) == args[0] }) else { die("no menu \(args[0])") }
    let items = children(top).flatMap { children($0) }
    if args[1] == "--list" {
        // Title, key equivalent and modifier mask (0 = Command only, 1 = Shift+Command) per item.
        emit(["menu": args[0], "items": items.map {
            [text($0, kAXTitleAttribute) ?? "", text($0, "AXMenuItemCmdChar") ?? "", text($0, "AXMenuItemCmdModifiers") ?? "",
             (attribute($0, kAXEnabledAttribute) as? Bool) == true ? "enabled" : "disabled"]
        }])
        break
    }
    guard let item = items.first(where: { text($0, kAXTitleAttribute) == args[1] }) else { die("no item \(args[1])") }
    var entry = assertFrontmost(t, "menu")
    entry["menu"] = [args[0], args[1]]
    entry["enabled"] = (attribute(item, kAXEnabledAttribute) as? Bool) ?? false
    let r = AXUIElementPerformAction(item, kAXPressAction as CFString)
    entry["ax_result"] = Int(r.rawValue)
    log(entry)
    emit(entry)

case "capture":
    let t = target()
    guard let out = option("--out") else { die("--out is required") }
    let window = reviewWindow(t.pid)
    let id = window["id"] as! Int
    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    process.arguments = ["-l\(id)", "-o", "-x", "-t", "png", out]
    try! process.run()
    process.waitUntilExit()
    guard process.terminationStatus == 0, FileManager.default.fileExists(atPath: out) else { die("screencapture failed", ["window": id]) }
    let entry: [String: Any] = ["action": "capture", "pid": t.pid, "exe": t.exe, "window": window, "out": out,
                                "pixels": pngSize(out), "sha256": sha256(out), "active": t.app.isActive]
    log(entry)
    emit(entry)

case "crop":
    // crop IN OUT X Y W H, in image pixels.
    guard args.count == 6, let x = Int(args[2]), let y = Int(args[3]), let w = Int(args[4]), let h = Int(args[5]) else { die("crop IN OUT X Y W H") }
    guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[0]) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
          let cropped = image.cropping(to: CGRect(x: x, y: y, width: w, height: h)),
          let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: args[1]) as CFURL, "public.png" as CFString, 1, nil) else { die("crop failed") }
    CGImageDestinationAddImage(dest, cropped, nil)
    guard CGImageDestinationFinalize(dest) else { die("crop write failed") }
    emit(["out": args[1], "pixels": [cropped.width, cropped.height], "sha256": sha256(args[1])])

case "pixel":
    // pixel IMG X Y ...: sRGB samples, for colour checks such as the account status dot.
    guard args.count >= 3, let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[0]) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { die("pixel IMG X Y ...") }
    let rep = NSBitmapImageRep(cgImage: image)
    var samples: [[String: Any]] = []
    var i = 1
    while i + 1 < args.count {
        let x = Int(args[i])!, y = Int(args[i + 1])!
        if let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) {
            samples.append(["x": x, "y": y, "hex": String(format: "#%02x%02x%02x", Int(round(c.redComponent * 255)), Int(round(c.greenComponent * 255)), Int(round(c.blueComponent * 255)))])
        }
        i += 2
    }
    emit(["samples": samples])

case "quit":
    // Stops only the verified review app: a normal quit request, then SIGTERM if it lingers.
    let t = target()
    var entry: [String: Any] = ["action": "quit", "pid": t.pid, "exe": t.exe, "kernel_path": kernelPath(t.pid) ?? "nil",
                                "bundle_path": t.app.bundleURL?.path ?? "nil"]
    entry["terminate_requested"] = t.app.terminate()
    for _ in 0..<60 where !t.app.isTerminated { usleep(100_000) }
    if !t.app.isTerminated, kernelPath(t.pid) == t.exe {
        entry["sigterm"] = kill(t.pid, SIGTERM) == 0
        for _ in 0..<30 where kernelPath(t.pid) != nil { usleep(100_000) }
    }
    entry["exited"] = kernelPath(t.pid) == nil
    log(entry)
    emit(entry)

default:
    die("unknown command \(command)")
}
