// winctl: window control for the 0044 native review, scoped to ONE process id.
//
// Every command names the PID of the incant_editor this review launched. Window
// lookups list only that PID's windows; input is posted only after that PID is
// frontmost, and every pointer location is checked to lie inside its main window.
// Captures use `screencapture -o -l <window>` on the id printed here, so no other
// app or screen content is ever recorded.
//
// Build: swiftc -O winctl.swift -o /tmp/incant0044/winctl
// Usage: winctl <pid> window                      -> {"id","x","y","w","h"} (points)
//        winctl <pid> raise                       -> brings the app and its window to front
//        winctl <pid> frame <x> <y> <w> <h>       -> moves/resizes the main window (AX)
//        winctl <pid> move <wx> <wy>              -> pointer to window-relative point
//        winctl <pid> click <wx> <wy> [left|right|middle] [count]
//        winctl <pid> down|up <wx> <wy> [left|right|middle]
//        winctl <pid> drag <wx> <wy> <wx2> <wy2>  -> left-drag in 12 steps
//        winctl <pid> scroll <wx> <wy> <dy> [pixel|line]
//        winctl <pid> key <keycode> [cmd,shift,alt,ctrl] [down|up]
//        winctl <pid> idle                        -> seconds since the last real input
//        winctl frontmost                         -> PID of the frontmost app
//        winctl cursor                            -> global pointer location, to restore after a step
//        winctl warp <x> <y>                      -> puts the pointer back (no events reach any app)
import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

func fail(_ message: String) -> Never {
  FileHandle.standardError.write((message + "\n").data(using: .utf8)!)
  exit(1)
}

func mainWindow(_ pid: pid_t) -> (id: CGWindowID, frame: CGRect) {
  guard let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else {
    fail("no window list")
  }
  let mine = list.filter { ($0[kCGWindowOwnerPID as String] as? pid_t) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }
  guard let window = mine.max(by: { area($0) < area($1) }),
        let id = window[kCGWindowNumber as String] as? CGWindowID,
        let bounds = window[kCGWindowBounds as String] as? NSDictionary,
        let frame = CGRect(dictionaryRepresentation: bounds) else {
    fail("pid \(pid) has no on-screen window")
  }
  return (id, frame)
}

func area(_ info: [String: Any]) -> CGFloat {
  guard let bounds = info[kCGWindowBounds as String] as? NSDictionary, let r = CGRect(dictionaryRepresentation: bounds) else { return 0 }
  return r.width * r.height
}

func axWindow(_ pid: pid_t) -> AXUIElement {
  let app = AXUIElementCreateApplication(pid)
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value) == .success,
        let windows = value as? [AXUIElement], let first = windows.first else {
    fail("pid \(pid) has no accessibility window")
  }
  return first
}

func frontmost() -> pid_t {
  NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1
}

/// Seconds since the last real (HID) input: someone using the Mac.
func idleSeconds() -> Double {
  CGEventSource.secondsSinceLastEventType(.hidSystemState, eventType: CGEventType(rawValue: ~0)!)
}

func bringToFront(_ pid: pid_t) {
  // Never take the front while someone is typing or pointing.
  guard idleSeconds() >= 8 else { fail("someone is using the Mac (idle \(Int(idleSeconds())) s); not taking the front") }
  // AX frontmost is refused once a person has switched apps; System Events is honoured.
  let script = "tell application \"System Events\" to set frontmost of (first process whose unix id is \(pid)) to true"
  var error: NSDictionary?
  NSAppleScript(source: script)?.executeAndReturnError(&error)
  AXUIElementPerformAction(axWindow(pid), kAXRaiseAction as CFString)
  for _ in 0..<40 where frontmost() != pid { usleep(25_000) }
  guard frontmost() == pid else { fail("pid \(pid) did not become frontmost") }
}

/// Window-relative point to a global point, refusing anything outside the window.
func point(_ pid: pid_t, _ wx: Double, _ wy: Double) -> CGPoint {
  let frame = mainWindow(pid).frame
  guard wx >= 0, wy >= 0, wx < frame.width, wy < frame.height else {
    fail("(\(wx), \(wy)) is outside the \(frame.width)x\(frame.height) window")
  }
  return CGPoint(x: frame.minX + wx, y: frame.minY + wy)
}

/// Input goes nowhere unless this review's app is in front.
func ensureFront(_ pid: pid_t) {
  if frontmost() != pid { bringToFront(pid) }
}

let source = CGEventSource(stateID: .hidSystemState)
/// The review's PID, checked again before every single event.
var target: pid_t = -1

func post(_ event: CGEvent?) {
  guard let event else { fail("could not create event") }
  // If another app came to the front (someone is using the Mac), stop rather than
  // deliver this event anywhere else.
  guard frontmost() == target else { fail("aborted: pid \(target) is no longer frontmost") }
  event.post(tap: .cghidEventTap)
  usleep(8_000)
}

func buttonTypes(_ name: String) -> (CGEventType, CGEventType, CGEventType, CGMouseButton) {
  switch name {
  case "right": return (.rightMouseDown, .rightMouseUp, .rightMouseDragged, .right)
  case "middle": return (.otherMouseDown, .otherMouseUp, .otherMouseDragged, .center)
  default: return (.leftMouseDown, .leftMouseUp, .leftMouseDragged, .left)
  }
}

func flags(_ list: String) -> CGEventFlags {
  var result: CGEventFlags = []
  for name in list.split(separator: ",") {
    switch name {
    case "cmd": result.insert(.maskCommand)
    case "shift": result.insert(.maskShift)
    case "alt": result.insert(.maskAlternate)
    case "ctrl": result.insert(.maskControl)
    case "none", "": break
    default: fail("unknown modifier \(name)")
    }
  }
  return result
}

let args = CommandLine.arguments.dropFirst().map { $0 }
if args.first == "frontmost" {
  print(frontmost())
  exit(0)
}
if args.first == "cursor" {
  let at = CGEvent(source: nil)?.location ?? .zero
  print("\(at.x) \(at.y)")
  exit(0)
}
if args.first == "warp", args.count == 3, let x = Double(args[1]), let y = Double(args[2]) {
  CGWarpMouseCursorPosition(CGPoint(x: x, y: y))
  exit(0)
}
guard args.count >= 2, let pid = pid_t(args[0]) else { fail("usage: winctl <pid> <command> ...") }
target = pid
let command = args[1]
let rest = Array(args.dropFirst(2))
func number(_ index: Int) -> Double {
  guard index < rest.count, let value = Double(rest[index]) else { fail("\(command): missing number \(index)") }
  return value
}

switch command {
case "window":
  let (id, frame) = mainWindow(pid)
  print("{\"id\":\(id),\"x\":\(frame.minX),\"y\":\(frame.minY),\"w\":\(frame.width),\"h\":\(frame.height)}")
case "raise":
  bringToFront(pid)
  print("front")
case "idle":
  print(Int(idleSeconds()))
case "frame":
  let window = axWindow(pid)
  var origin = CGPoint(x: number(0), y: number(1))
  var size = CGSize(width: number(2), height: number(3))
  guard let position = AXValueCreate(.cgPoint, &origin), let extent = AXValueCreate(.cgSize, &size) else { fail("AX value") }
  AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, position)
  AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, extent)
  usleep(300_000)
  let frame = mainWindow(pid).frame
  print("{\"x\":\(frame.minX),\"y\":\(frame.minY),\"w\":\(frame.width),\"h\":\(frame.height)}")
case "move":
  ensureFront(pid)
  post(CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: point(pid, number(0), number(1)), mouseButton: .left))
case "click", "down", "up":
  ensureFront(pid)
  let at = point(pid, number(0), number(1))
  let (down, up, _, button) = buttonTypes(rest.count > 2 ? rest[2] : "left")
  let count = rest.count > 3 ? Int(rest[3]) ?? 1 : 1
  for click in 1...count {
    if command != "up" {
      let event = CGEvent(mouseEventSource: source, mouseType: down, mouseCursorPosition: at, mouseButton: button)
      event?.setIntegerValueField(.mouseEventClickState, value: Int64(click))
      post(event)
    }
    if command != "down" {
      let event = CGEvent(mouseEventSource: source, mouseType: up, mouseCursorPosition: at, mouseButton: button)
      event?.setIntegerValueField(.mouseEventClickState, value: Int64(click))
      post(event)
    }
  }
case "drag":
  ensureFront(pid)
  let from = point(pid, number(0), number(1))
  let to = point(pid, number(2), number(3))
  post(CGEvent(mouseEventSource: source, mouseType: .leftMouseDown, mouseCursorPosition: from, mouseButton: .left))
  for step in 1...12 {
    let t = Double(step) / 12
    let at = CGPoint(x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t)
    post(CGEvent(mouseEventSource: source, mouseType: .leftMouseDragged, mouseCursorPosition: at, mouseButton: .left))
  }
  post(CGEvent(mouseEventSource: source, mouseType: .leftMouseUp, mouseCursorPosition: to, mouseButton: .left))
case "scroll":
  ensureFront(pid)
  let at = point(pid, number(0), number(1))
  post(CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: at, mouseButton: .left))
  let unit: CGScrollEventUnit = rest.count > 3 && rest[3] == "line" ? .line : .pixel
  post(CGEvent(scrollWheelEvent2Source: source, units: unit, wheelCount: 1, wheel1: Int32(number(2)), wheel2: 0, wheel3: 0))
case "key":
  ensureFront(pid)
  guard let code = rest.first.flatMap({ CGKeyCode($0) }) else { fail("key: missing keycode") }
  let modifiers = flags(rest.count > 1 ? rest[1] : "")
  let phase = rest.count > 2 ? rest[2] : "press"
  for down in phase == "down" ? [true] : phase == "up" ? [false] : [true, false] {
    let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down)
    event?.flags = modifiers
    post(event)
  }
default:
  fail("unknown command \(command)")
}
