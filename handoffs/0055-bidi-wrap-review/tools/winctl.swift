// winctl: window control for the 0055 bidi wrap review, scoped to ONE process id.
//
// Same behavior as the 0054 copy (reduced from the 0050/0053 tool). Window lookups list only the
// given PID's windows; nothing here posts keyboard or pointer events into any app. `raise` refuses unless the
// Mac has been idle, and captures use `screencapture -o -l <window>` on the id printed here, so no
// other app or screen content is recorded.
//
// Build: swiftc -O winctl.swift -o /tmp/incant0055/winctl
// Usage: winctl <pid> window                -> {"id","x","y","w","h"} (points)
//        winctl <pid> raise                 -> brings that app and its window to front
//        winctl <pid> frame <x> <y> <w> <h> -> moves/resizes its standard window through AX
//        winctl <pid> close                 -> presses that window's own close button (AX)
//        winctl idle                        -> seconds since the last real input
//        winctl frontmost                   -> PID of the frontmost app
//        winctl cursor                      -> global pointer location, to restore after a step
//        winctl warp <x> <y>                -> puts the pointer back (no events reach any app)
//        winctl screen                      -> main screen frame, visible frame and scale
//        winctl hold <seconds>              -> waits; exits 7 at once if real input arrives meanwhile
import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

func fail(_ message: String) -> Never {
  FileHandle.standardError.write((message + "\n").data(using: .utf8)!)
  exit(1)
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

func idleSeconds() -> Double {
  CGEventSource.secondsSinceLastEventType(.hidSystemState, eventType: CGEventType(rawValue: ~0)!)
}

func bringToFront(_ pid: pid_t) {
  guard idleSeconds() >= 8 else { fail("someone is using the Mac (idle \(Int(idleSeconds())) s); not taking the front") }
  let script = "tell application \"System Events\" to set frontmost of (first process whose unix id is \(pid)) to true"
  var error: NSDictionary?
  NSAppleScript(source: script)?.executeAndReturnError(&error)
  AXUIElementPerformAction(axWindow(pid), kAXRaiseAction as CFString)
  for _ in 0..<40 where frontmost() != pid { usleep(25_000) }
  guard frontmost() == pid else { fail("pid \(pid) did not become frontmost") }
}

let args = Array(CommandLine.arguments.dropFirst())
switch args.first {
case "frontmost":
  print(frontmost())
  exit(0)
case "cursor":
  let at = CGEvent(source: nil)?.location ?? .zero
  print("\(at.x) \(at.y)")
  exit(0)
case "warp" where args.count == 3:
  guard let x = Double(args[1]), let y = Double(args[2]) else { fail("warp needs numbers") }
  CGWarpMouseCursorPosition(CGPoint(x: x, y: y))
  exit(0)
case "idle":
  print(Int(idleSeconds()))
  exit(0)
case "hold" where args.count == 2:
  // Yield to the user: any real input during the hold resets the idle clock below the
  // time already waited, and the caller hands the front back without capturing.
  guard let seconds = Double(args[1]), seconds >= 0 else { fail("hold needs seconds") }
  let start = Date(), before = idleSeconds()
  while Date().timeIntervalSince(start) < seconds {
    usleep(100_000)
    let waited = Date().timeIntervalSince(start)
    if idleSeconds() + 0.05 < before + waited - 0.3 { fail("user input during hold; yielding") }
  }
  exit(0)
case "screen":
  guard let screen = NSScreen.main else { fail("no main screen") }
  let f = screen.frame, v = screen.visibleFrame
  print("{\"frame\":[\(f.minX),\(f.minY),\(f.width),\(f.height)],\"visible\":[\(v.minX),\(v.minY),\(v.width),\(v.height)],\"scale\":\(screen.backingScaleFactor)}")
  exit(0)
default:
  break
}
guard args.count >= 2, let pid = pid_t(args[0]) else { fail("usage: winctl <pid> window|raise|frame ...") }
switch args[1] {
case "window":
  let (id, frame) = mainWindow(pid)
  print("{\"id\":\(id),\"x\":\(frame.minX),\"y\":\(frame.minY),\"w\":\(frame.width),\"h\":\(frame.height)}")
case "raise":
  bringToFront(pid)
  print("front")
case "frame" where args.count == 6:
  let values = args[2...].compactMap(Double.init)
  guard values.count == 4 else { fail("frame needs x y w h") }
  var origin = CGPoint(x: values[0], y: values[1])
  var size = CGSize(width: values[2], height: values[3])
  guard let position = AXValueCreate(.cgPoint, &origin), let extent = AXValueCreate(.cgSize, &size) else { fail("AX value") }
  let window = axWindow(pid)
  AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, position)
  AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, extent)
  usleep(400_000)
  let frame = mainWindow(pid).frame
  print("{\"x\":\(frame.minX),\"y\":\(frame.minY),\"w\":\(frame.width),\"h\":\(frame.height)}")
case "close":
  var button: CFTypeRef?
  guard AXUIElementCopyAttributeValue(axWindow(pid), kAXCloseButtonAttribute as CFString, &button) == .success,
        let close = button else { fail("pid \(pid) window has no close button") }
  guard AXUIElementPerformAction(close as! AXUIElement, kAXPressAction as CFString) == .success else {
    fail("close button press failed")
  }
  print("closed")
default:
  fail("unknown command \(args[1])")
}
