// winctl: native window control for the 0060 Core Sample review, scoped to ONE process id.
//
// Adapted from the earlier native review tool (read as reference only; no earlier tool, marker or
// process is reopened) with 0060 paths, its own lease guard and held-key commands for movement.
// EVERY command, read-only queries included, refuses (exit 2) after the 0060 release marker,
// without the 0060 lease or once the lease has expired.
//
// Window lookups list only the given PID's windows. Input is posted only while that PID is
// frontmost (rechecked before every event) and the session is unlocked with the display awake;
// every pointer location must lie inside its window's content area, and an input step aborts
// if the pointer moved since this tool last placed it (a person is using the mouse). Nothing
// here changes system settings: the input source list is read only.
//
// Native event mechanisms used (macOS):
//   pointer   CGEvent mouseMoved/leftMouseDown/leftMouseUp posted at kCGHIDEventTap
//   keys      CGEvent keyDown/keyUp with US-layout virtual key codes posted at kCGHIDEventTap;
//             modifiers are real modifier key events (Shift 56, Option 58) plus event flags
//   unicode   CGEvent keyDown/keyUp carrying a Unicode string (keyboardSetUnicodeString)
//   frame     AX kAXPositionAttribute/kAXSizeAttribute on the standard window
//   close     the window's own AX close button (the player then exits through CloseRequested)
//   focus     System Events "set frontmost" of a process (app activation and hand-back)
//
// Build: tools/setup.sh (swiftc -O winctl.swift -o ~/.codex/incant-evidence/0060/winctl)
// Usage: winctl <pid> window                 -> {"id","x","y","w","h"} (points, frame incl. title bar)
//        winctl <pid> raise                  -> brings that app and its window to front (idle >= 8 s)
//        winctl <pid> frame <x> <y> <w> <h>  -> moves/resizes its standard window through AX
//        winctl <pid> close                  -> presses that window's own close button (AX)
//        winctl <pid> scale                  -> pixel scale of the display under the window centre
//        winctl <pid> move|click|down|up <cx> <cy> -> pointer at a content point (points)
//        winctl <pid> key <code> [shift|alt|none]  -> one key press (US virtual key code)
//        winctl <pid> keys <text>            -> US-layout key codes for [a-zA-Z0-9 .,-]
//        winctl <pid> keydown|keyup <code>   -> one key edge (hold movement keys between them)
//        winctl <pid> unicode <text>         -> one Unicode-string key event pair per character
//        winctl activate <pid>               -> System Events: that process to front (hand back)
//        winctl idle | frontmost | cursor | warp <x> <y> | locked | displays | sources | hold <s>
import AppKit
import ApplicationServices
import Carbon
import CoreGraphics
import Foundation

/// The packet's evidence directory (0060 lease, title bar measurement, pointer guard).
let evidence = NSHomeDirectory() + "/.codex/incant-evidence/0060"

func fail(_ message: String, _ code: Int32 = 1) -> Never {
  FileHandle.standardError.write((message + "\n").data(using: .utf8)!)
  exit(code)
}

/// The packet lease: present, this packet's, unexpired and not released.
func leaseGuard() {
  let files = FileManager.default
  if files.fileExists(atPath: evidence + "/released") { fail("0060 lease was released; refusing", 2) }
  guard let data = files.contents(atPath: evidence + "/lease.json"),
        let lease = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
        lease["packet"] as? String == "0060-core-sample-presentation",
        let text = lease["expires_at"] as? String,
        let expiry = ISO8601DateFormatter().date(from: text) else {
    fail("no valid 0060 lease; refusing", 2)
  }
  guard Date() < expiry else { fail("the 0060 lease expired at \(text); refusing", 2) }
}

func area(_ info: [String: Any]) -> CGFloat {
  guard let bounds = info[kCGWindowBounds as String] as? NSDictionary,
        let r = CGRect(dictionaryRepresentation: bounds) else { return 0 }
  return r.width * r.height
}

func mainWindow(_ pid: pid_t) -> (id: CGWindowID, frame: CGRect) {
  guard let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
    as? [[String: Any]] else { fail("no window list") }
  let mine = list.filter {
    ($0[kCGWindowOwnerPID as String] as? pid_t) == pid && ($0[kCGWindowLayer as String] as? Int) == 0
  }
  guard let window = mine.max(by: { area($0) < area($1) }),
        let id = window[kCGWindowNumber as String] as? CGWindowID,
        let bounds = window[kCGWindowBounds as String] as? NSDictionary,
        let frame = CGRect(dictionaryRepresentation: bounds) else { fail("pid \(pid) has no on-screen window") }
  return (id, frame)
}

func axWindow(_ pid: pid_t) -> AXUIElement {
  let app = AXUIElementCreateApplication(pid)
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value) == .success,
        let windows = value as? [AXUIElement] else { fail("pid \(pid) has no accessibility window") }
  let standard = windows.filter {
    var role: CFTypeRef?
    AXUIElementCopyAttributeValue($0, kAXSubroleAttribute as CFString, &role)
    return (role as? String) == "AXStandardWindow"
  }
  guard let window = standard.first else { fail("pid \(pid) has no standard accessibility window") }
  return window
}

func frontmost() -> pid_t { NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1 }

/// A locked session or sleeping display means nobody can see the window and any key would reach
/// the lock screen: every input, raise and capture refuses.
func locked() -> Bool {
  let session = CGSessionCopyCurrentDictionary() as? [String: Any]
  let screen = (session?["CGSSessionScreenIsLocked"] as? NSNumber)?.boolValue ?? false
  return screen || CGDisplayIsAsleep(CGMainDisplayID()) != 0
}

func idleSeconds() -> Double {
  CGEventSource.secondsSinceLastEventType(.hidSystemState, eventType: CGEventType(rawValue: ~0)!)
}

func activate(_ pid: pid_t) {
  let script = "tell application \"System Events\" to set frontmost of (first process whose unix id is \(pid)) to true"
  var error: NSDictionary?
  NSAppleScript(source: script)?.executeAndReturnError(&error)
}

func bringToFront(_ pid: pid_t) {
  guard !locked() else { fail("the session is locked or the display is asleep; not raising", 8) }
  guard idleSeconds() >= 8 else { fail("someone is using the Mac (idle \(Int(idleSeconds())) s); not taking the front", 7) }
  activate(pid)
  AXUIElementPerformAction(axWindow(pid), kAXRaiseAction as CFString)
  for _ in 0..<40 where frontmost() != pid { usleep(25_000) }
  guard frontmost() == pid else { fail("pid \(pid) did not become frontmost") }
}

/// The content area origin: frame top-left plus the measured title bar height.
func titlebar() -> Double {
  guard let text = try? String(contentsOfFile: evidence + "/titlebar.pt", encoding: .utf8),
        let value = Double(text.trimmingCharacters(in: .whitespacesAndNewlines)) else {
    fail("title bar height not measured yet (launch.sh writes titlebar.pt)")
  }
  return value
}

/// Content-relative point (points) to a global point, refusing anything outside the content.
func point(_ pid: pid_t, _ cx: Double, _ cy: Double) -> CGPoint {
  let frame = mainWindow(pid).frame
  let bar = titlebar()
  guard cx >= 0, cy >= 0, cx < frame.width, cy < frame.height - bar else {
    fail("(\(cx), \(cy)) is outside the \(frame.width)x\(frame.height - bar) content")
  }
  return CGPoint(x: frame.minX + cx, y: frame.minY + bar + cy)
}

let source = CGEventSource(stateID: .hidSystemState)
var target: pid_t = -1
let lastPointer = evidence + "/pointer.last"

func pointerNow() -> CGPoint { CGEvent(source: nil)?.location ?? .zero }

/// Someone moved the mouse since this tool last placed it: stop.
func checkPointer() {
  guard let text = try? String(contentsOfFile: lastPointer, encoding: .utf8) else { return }
  let parts = text.split(separator: " ").compactMap { Double($0) }
  guard parts.count == 2 else { return }
  let now = pointerNow()
  if abs(now.x - parts[0]) > 1 || abs(now.y - parts[1]) > 1 {
    fail("aborted: the pointer moved (\(now.x), \(now.y)); someone is using the Mac", 7)
  }
}

func remember(_ at: CGPoint) {
  try? "\(at.x) \(at.y)".write(toFile: lastPointer, atomically: true, encoding: .utf8)
}

/// Every event rechecks the lock and that the scoped PID owns the front.
func post(_ event: CGEvent?) {
  guard let event else { fail("could not create event") }
  guard !locked() else { fail("aborted: the session is locked or the display is asleep", 8) }
  guard frontmost() == target else { fail("aborted: pid \(target) is no longer frontmost", 7) }
  event.post(tap: .cghidEventTap)
  usleep(12_000)
}

func mouse(_ type: CGEventType, _ at: CGPoint) {
  post(CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: at, mouseButton: .left))
  remember(at)
}

/// Real modifier key event (flagsChanged) so the app sees the physical modifier key.
func modifier(_ name: String, _ down: Bool) {
  let (code, flag): (CGKeyCode, CGEventFlags) = name == "shift" ? (56, .maskShift) : (58, .maskAlternate)
  let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)
  event?.type = .flagsChanged
  event?.flags = down ? flag : []
  post(event)
}

func key(_ code: CGKeyCode, _ mods: String) {
  guard ["shift", "alt", "none"].contains(mods) else { fail("key: unknown modifier \(mods)") }
  let flag: CGEventFlags = mods == "shift" ? .maskShift : mods == "alt" ? .maskAlternate : []
  if mods != "none" { modifier(mods, true) }
  for down in [true, false] {
    let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)
    event?.flags = flag
    post(event)
    usleep(20_000)
  }
  if mods != "none" { modifier(mods, false) }
}

// US ANSI layout virtual key codes.
let letters: [Character: CGKeyCode] = [
  "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9, "b": 11, "q": 12,
  "w": 13, "e": 14, "r": 15, "y": 16, "t": 17, "1": 18, "2": 19, "3": 20, "4": 21, "6": 22, "5": 23,
  "9": 25, "7": 26, "-": 27, "8": 28, "0": 29, "o": 31, "u": 32, "i": 34, "p": 35, "l": 37, "j": 38,
  "k": 40, ",": 43, "n": 45, "m": 46, ".": 47, " ": 49,
]

func sources() {
  guard let list = TISCreateInputSourceList(nil, false)?.takeRetainedValue() as? [TISInputSource] else {
    fail("no input source list")
  }
  func property(_ source: TISInputSource, _ key: CFString) -> String {
    guard let raw = TISGetInputSourceProperty(source, key) else { return "" }
    return Unmanaged<AnyObject>.fromOpaque(raw).takeUnretainedValue() as? String ?? ""
  }
  let current = TISCopyCurrentKeyboardInputSource()?.takeRetainedValue()
  for item in list {
    let id = property(item, kTISPropertyInputSourceID)
    let category = property(item, kTISPropertyInputSourceCategory)
    let selected = current.map { property($0, kTISPropertyInputSourceID) == id } ?? false
    print("{\"id\":\"\(id)\",\"category\":\"\(category)\",\"current_keyboard\":\(selected)}")
  }
}

leaseGuard()
let args = Array(CommandLine.arguments.dropFirst())
switch args.first {
case "frontmost": print(frontmost()); exit(0)
case "cursor": let at = pointerNow(); print("\(at.x) \(at.y)"); exit(0)
case "warp" where args.count == 3:
  guard let x = Double(args[1]), let y = Double(args[2]) else { fail("warp needs numbers") }
  CGWarpMouseCursorPosition(CGPoint(x: x, y: y))
  try? FileManager.default.removeItem(atPath: lastPointer)
  exit(0)
case "idle": print(Int(idleSeconds())); exit(0)
case "locked": print(locked() ? 1 : 0); exit(0)
case "displays":
  var count: UInt32 = 0
  CGGetActiveDisplayList(0, nil, &count)
  var ids = [CGDirectDisplayID](repeating: 0, count: Int(count))
  CGGetActiveDisplayList(count, &ids, &count)
  for id in ids {
    let b = CGDisplayBounds(id)
    let scale = CGDisplayCopyDisplayMode(id).map { Double($0.pixelWidth) / Double($0.width) } ?? 0
    print("{\"id\":\(id),\"main\":\(CGDisplayIsMain(id) != 0),\"x\":\(b.minX),\"y\":\(b.minY),\"w\":\(b.width),\"h\":\(b.height),\"scale\":\(scale)}")
  }
  exit(0)
case "sources": sources(); exit(0)
case "activate" where args.count == 2:
  guard let pid = pid_t(args[1]) else { fail("activate needs a pid") }
  guard !locked() else { fail("the session is locked or the display is asleep; not activating", 8) }
  activate(pid)
  for _ in 0..<40 where frontmost() != pid { usleep(25_000) }
  print(frontmost())
  exit(0)
case "hold" where args.count == 2:
  // Yield to the user: real input during the hold resets the idle clock below the time
  // already waited; the caller then hands the front back without capturing.
  guard let seconds = Double(args[1]), seconds >= 0 else { fail("hold needs seconds") }
  let start = Date(), before = idleSeconds()
  while Date().timeIntervalSince(start) < seconds {
    usleep(50_000)
    let waited = Date().timeIntervalSince(start)
    if idleSeconds() + 0.05 < before + waited - 0.3 { fail("user input during hold; yielding", 7) }
  }
  exit(0)
default: break
}
guard args.count >= 2, let pid = pid_t(args[0]) else { fail("usage: winctl <pid> <command> ...") }
target = pid
let rest = Array(args.dropFirst(2))
func number(_ i: Int) -> Double {
  guard i < rest.count, let value = Double(rest[i]) else { fail("\(args[1]): missing number \(i)") }
  return value
}
switch args[1] {
case "window":
  let (id, frame) = mainWindow(pid)
  print("{\"id\":\(id),\"x\":\(frame.minX),\"y\":\(frame.minY),\"w\":\(frame.width),\"h\":\(frame.height)}")
case "raise":
  bringToFront(pid)
  print("front")
case "frame":
  guard !locked() else { fail("the session is locked or the display is asleep; not moving", 8) }
  var origin = CGPoint(x: number(0), y: number(1))
  var size = CGSize(width: number(2), height: number(3))
  guard let position = AXValueCreate(.cgPoint, &origin), let extent = AXValueCreate(.cgSize, &size) else { fail("AX value") }
  let window = axWindow(pid)
  AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, position)
  AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, extent)
  usleep(400_000)
  let frame = mainWindow(pid).frame
  print("{\"x\":\(frame.minX),\"y\":\(frame.minY),\"w\":\(frame.width),\"h\":\(frame.height)}")
case "scale":
  let frame = mainWindow(pid).frame
  var display: CGDirectDisplayID = 0
  var count: UInt32 = 0
  CGGetDisplaysWithPoint(CGPoint(x: frame.midX, y: frame.midY), 1, &display, &count)
  guard count == 1, let mode = CGDisplayCopyDisplayMode(display) else { fail("no display under the window") }
  print(Double(mode.pixelWidth) / Double(mode.width))
case "close":
  var button: CFTypeRef?
  guard AXUIElementCopyAttributeValue(axWindow(pid), kAXCloseButtonAttribute as CFString, &button) == .success,
        let close = button else { fail("pid \(pid) window has no close button") }
  guard AXUIElementPerformAction(close as! AXUIElement, kAXPressAction as CFString) == .success else {
    fail("close button press failed")
  }
  print("closed")
case "move":
  checkPointer()
  mouse(.mouseMoved, point(pid, number(0), number(1)))
case "click", "down", "up":
  checkPointer()
  let at = point(pid, number(0), number(1))
  if pointerNow() != at { mouse(.mouseMoved, at); usleep(30_000) }
  if args[1] != "up" { mouse(.leftMouseDown, at) }
  if args[1] == "click" { usleep(60_000) }
  if args[1] != "down" { mouse(.leftMouseUp, at) }
case "key":
  guard let code = rest.first.flatMap({ CGKeyCode($0) }) else { fail("key: missing keycode") }
  checkPointer()
  key(code, rest.count > 1 ? rest[1] : "none")
case "keydown", "keyup":
  guard let code = rest.first.flatMap({ CGKeyCode($0) }) else { fail("\(args[1]): missing keycode") }
  checkPointer()
  post(CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: args[1] == "keydown"))
case "keys":
  guard let text = rest.first else { fail("keys: missing text") }
  checkPointer()
  for character in text {
    let lower = Character(character.lowercased())
    guard let code = letters[lower] else { fail("keys: no US key code for \(character)") }
    key(code, character.isUppercase ? "shift" : "none")
  }
case "unicode":
  guard let text = rest.first else { fail("unicode: missing text") }
  checkPointer()
  for character in text {
    var units = Array(String(character).utf16)
    for down in [true, false] {
      let event = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: down)
      event?.keyboardSetUnicodeString(stringLength: units.count, unicodeString: &units)
      post(event)
      usleep(25_000)
    }
  }
default:
  fail("unknown command \(args[1])")
}
