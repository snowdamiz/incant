// winctl: window control for the 0050 focused native review, scoped to ONE process id.
//
// Derived from handoffs 0044 and 0048. Every command names the PID of the incant_editor this
// review launched. Window lookups list only that PID's windows; input is posted only after that
// PID is frontmost (rechecked before every event), and every pointer location is checked to lie
// inside its main window. Captures use `screencapture -o -l <window>` on the id printed here, so
// no other app or screen content is recorded.
//
// Read-only accessibility queries of the same PID's window (role, name, value, focus and frame),
// the WebKit text selection, and typed text came in 0048. New in 0050: `ax attr` reads one named
// attribute, `ax bounds` reads the on-screen bounds of a character range of a text field, and
// `ax focuspath` prints the focused element and its ancestors. Account names are redacted from
// every AX string this tool prints.
//
// Build: swiftc -O winctl.swift -o /tmp/incant0050/winctl
// Usage: winctl <pid> window                      -> {"id","x","y","w","h"} (points)
//        winctl <pid> raise                       -> brings the app and its window to front
//        winctl <pid> frame <x> <y> <w> <h>       -> moves/resizes the main window (AX)
//        winctl <pid> move <wx> <wy>              -> pointer to window-relative point
//        winctl <pid> click <wx> <wy> [left|right|middle] [count]
//        winctl <pid> ctrlclick <wx> <wy>         -> left click with Control held
//        winctl <pid> down|up <wx> <wy> [left|right|middle]
//        winctl <pid> drag <wx> <wy> <wx2> <wy2> [steps] [hold-ms] -> left-drag
//        winctl <pid> scroll <wx> <wy> <dy> [pixel|line]
//        winctl <pid> key <keycode> [cmd,shift,alt,ctrl] [down|up]
//        winctl <pid> type <text>                 -> unicode text, one key event pair per character
//        winctl <pid> ax dump [maxDepth]          -> JSON lines of the window's AX tree
//        winctl <pid> ax focused                  -> the app's focused AX element
//        winctl <pid> ax find <role|*> <text>     -> elements whose name/value/help contains text
//        winctl <pid> ax click <role|*> <text> [index] [left|right]
//        winctl <pid> ax selection                -> the web area's selected text (WebKit)
//        winctl <pid> ax attr <role|*> <text> <attribute> [index] -> one attribute's value
//        winctl <pid> ax bounds <role|*> <text> <location> <length> [index]
//                                                 -> window-relative bounds of that character range
//        winctl <pid> ax focuspath                -> the focused element, then each ancestor
//        winctl <pid> idle                        -> seconds since the last real input
//        winctl frontmost                         -> PID of the frontmost app
//        winctl cursor                            -> global pointer location, to restore after a step
//        winctl warp <x> <y>                      -> puts the pointer back (no events reach any app)
//        winctl screen                            -> main screen frame and visible frame (points)
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

/// The app's standard window. Tooltips (AXHelpTag) and menus are windows too, and can come first.
func axWindow(_ pid: pid_t) -> AXUIElement {
  let app = AXUIElementCreateApplication(pid)
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value) == .success,
        let windows = value as? [AXUIElement] else {
    fail("pid \(pid) has no accessibility window")
  }
  let standard = windows.filter {
    var role: CFTypeRef?
    AXUIElementCopyAttributeValue($0, kAXSubroleAttribute as CFString, &role)
    return (role as? String) == "AXStandardWindow"
  }
  guard let window = standard.first else { fail("pid \(pid) has no standard accessibility window") }
  return window
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

// MARK: accessibility (read-only queries of the review PID)

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
  var value: CFTypeRef?
  return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}

func text(_ element: AXUIElement, _ name: String) -> String? {
  guard let value = attribute(element, name) else { return nil }
  if let string = value as? String { return string.isEmpty ? nil : string }
  if CFGetTypeID(value) == CFBooleanGetTypeID() { return (value as! Bool) ? "true" : "false" }
  if let number = value as? NSNumber { return number.stringValue }
  return nil
}

func flag(_ element: AXUIElement, _ name: String) -> Bool? {
  guard let value = attribute(element, name), CFGetTypeID(value) == CFBooleanGetTypeID() else { return nil }
  return (value as! Bool)
}

func frameOf(_ element: AXUIElement) -> CGRect? {
  guard let position = attribute(element, kAXPositionAttribute as String),
        let size = attribute(element, kAXSizeAttribute as String),
        CFGetTypeID(position) == AXValueGetTypeID(), CFGetTypeID(size) == AXValueGetTypeID() else { return nil }
  var origin = CGPoint.zero
  var extent = CGSize.zero
  AXValueGetValue(position as! AXValue, .cgPoint, &origin)
  AXValueGetValue(size as! AXValue, .cgSize, &extent)
  return CGRect(origin: origin, size: extent)
}

func children(_ element: AXUIElement) -> [AXUIElement] {
  (attribute(element, kAXChildrenAttribute as String) as? [AXUIElement]) ?? []
}

/// Account identities never leave this tool: the account button's name and any e-mail address.
func redact(_ value: String) -> String {
  var result = value
  if let range = result.range(of: "signed in as", options: .caseInsensitive) {
    result = String(result[..<range.upperBound]) + " [redacted]"
  }
  let email = try! NSRegularExpression(pattern: "[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}")
  result = email.stringByReplacingMatches(in: result, range: NSRange(result.startIndex..., in: result), withTemplate: "[redacted e-mail]")
  return result
}

func json(_ value: String) -> String {
  let data = try! JSONSerialization.data(withJSONObject: [value], options: [.withoutEscapingSlashes])
  let array = String(data: data, encoding: .utf8)!
  return String(array.dropFirst().dropLast())
}

/// WINCTL_FULL=1 prints AX strings untruncated (used to read exact approval arguments).
let limit = ProcessInfo.processInfo.environment["WINCTL_FULL"] == "1" ? Int.max : 200

struct Node {
  let element: AXUIElement
  let depth: Int
  let role: String
  let subrole: String?
  let title: String?
  let label: String?
  let value: String?
  let help: String?
  let frame: CGRect?
  let focused: Bool?
  let enabled: Bool?
  let domId: String?

  var searchable: String { [title, label, value, help].compactMap { $0 }.joined(separator: " ") }

  func line(relativeTo window: CGRect) -> String {
    var fields = ["\"d\":\(depth)", "\"role\":\(json(role))"]
    if let subrole { fields.append("\"sub\":\(json(subrole))") }
    if let title { fields.append("\"title\":\(json(redact(String(title.prefix(limit)))))") }
    if let label { fields.append("\"label\":\(json(redact(String(label.prefix(limit)))))") }
    if let value { fields.append("\"value\":\(json(redact(String(value.prefix(limit)))))") }
    if let help { fields.append("\"help\":\(json(redact(String(help.prefix(limit)))))") }
    if let frame {
      let x = frame.minX - window.minX, y = frame.minY - window.minY
      fields.append(String(format: "\"x\":%.1f,\"y\":%.1f,\"w\":%.1f,\"h\":%.1f", x, y, frame.width, frame.height))
    }
    if focused == true { fields.append("\"focused\":true") }
    if enabled == false { fields.append("\"enabled\":false") }
    if let domId { fields.append("\"dom\":\(json(domId))") }
    return "{" + fields.joined(separator: ",") + "}"
  }
}

func node(_ element: AXUIElement, depth: Int) -> Node {
  Node(element: element, depth: depth,
       role: text(element, kAXRoleAttribute as String) ?? "?",
       subrole: text(element, kAXSubroleAttribute as String),
       title: text(element, kAXTitleAttribute as String),
       label: text(element, kAXDescriptionAttribute as String),
       value: text(element, kAXValueAttribute as String),
       help: text(element, kAXHelpAttribute as String),
       frame: frameOf(element),
       focused: flag(element, kAXFocusedAttribute as String),
       enabled: flag(element, kAXEnabledAttribute as String),
       domId: text(element, "AXDOMIdentifier"))
}

func walk(_ element: AXUIElement, depth: Int, maxDepth: Int, visit: (Node) -> Void) {
  visit(node(element, depth: depth))
  guard depth < maxDepth else { return }
  for child in children(element) { walk(child, depth: depth + 1, maxDepth: maxDepth, visit: visit) }
}

/// A needle starting with "=" must equal the title or label exactly; otherwise it is a
/// case-insensitive substring of the title, label, value or help.
func matches(_ pid: pid_t, role: String, needle: String) -> [Node] {
  var found: [Node] = []
  let exact = needle.hasPrefix("=") ? String(needle.dropFirst()) : nil
  walk(axWindow(pid), depth: 0, maxDepth: 60) { item in
    guard role == "*" || item.role == role else { return }
    if let exact {
      if item.title == exact || item.label == exact { found.append(item) }
    } else if item.searchable.range(of: needle, options: .caseInsensitive) != nil {
      found.append(item)
    }
  }
  return found
}

/// A readable form of any attribute value: text, numbers, ranges, geometry, elements, lists.
func describe(_ value: CFTypeRef?, window: CGRect) -> String {
  guard let value else { return "null" }
  let type = CFGetTypeID(value)
  if type == AXValueGetTypeID() {
    let ax = value as! AXValue
    switch AXValueGetType(ax) {
    case .cfRange:
      var range = CFRange()
      AXValueGetValue(ax, .cfRange, &range)
      return "{\"location\":\(range.location),\"length\":\(range.length)}"
    case .cgRect:
      var rect = CGRect.zero
      AXValueGetValue(ax, .cgRect, &rect)
      return String(format: "{\"x\":%.2f,\"y\":%.2f,\"w\":%.2f,\"h\":%.2f}", rect.minX - window.minX, rect.minY - window.minY, rect.width, rect.height)
    case .cgPoint:
      var point = CGPoint.zero
      AXValueGetValue(ax, .cgPoint, &point)
      return String(format: "{\"x\":%.2f,\"y\":%.2f}", point.x - window.minX, point.y - window.minY)
    case .cgSize:
      var size = CGSize.zero
      AXValueGetValue(ax, .cgSize, &size)
      return String(format: "{\"w\":%.2f,\"h\":%.2f}", size.width, size.height)
    default:
      return "\"<AXValue>\""
    }
  }
  if type == AXUIElementGetTypeID() { return node(value as! AXUIElement, depth: 0).line(relativeTo: window) }
  if type == CFBooleanGetTypeID() { return (value as! Bool) ? "true" : "false" }
  if let number = value as? NSNumber { return number.stringValue }
  if let string = value as? String { return json(redact(String(string.prefix(limit)))) }
  if let list = value as? [AnyObject] { return "{\"count\":\(list.count)}" }
  return "\"<\(String(describing: CFCopyTypeIDDescription(type)))>\""
}

func webArea(_ pid: pid_t) -> AXUIElement? {
  var area: AXUIElement?
  walk(axWindow(pid), depth: 0, maxDepth: 12) { item in
    if area == nil && item.role == "AXWebArea" { area = item.element }
  }
  return area
}

func axCommand(_ pid: pid_t, _ rest: [String]) {
  guard let sub = rest.first else { fail("ax: missing subcommand") }
  let window = mainWindow(pid).frame
  switch sub {
  case "dump":
    let maxDepth = rest.count > 1 ? Int(rest[1]) ?? 60 : 60
    walk(axWindow(pid), depth: 0, maxDepth: maxDepth) { print($0.line(relativeTo: window)) }
  case "focused":
    let app = AXUIElementCreateApplication(pid)
    guard let value = attribute(app, kAXFocusedUIElementAttribute as String),
          CFGetTypeID(value) == AXUIElementGetTypeID() else { print("{\"focused\":null}"); return }
    print(node(value as! AXUIElement, depth: 0).line(relativeTo: window))
  case "find":
    guard rest.count >= 3 else { fail("ax find <role|*> <text>") }
    for item in matches(pid, role: rest[1], needle: rest[2]) { print(item.line(relativeTo: window)) }
  case "click":
    guard rest.count >= 3 else { fail("ax click <role|*> <text> [index] [button]") }
    let found = matches(pid, role: rest[1], needle: rest[2])
    let index = rest.count > 3 ? Int(rest[3]) ?? 0 : 0
    guard index < found.count, let frame = found[index].frame else {
      fail("ax click: \(found.count) match(es) for \(rest[1]) \"\(rest[2])\"; index \(index) unavailable")
    }
    ensureFront(pid)
    let at = point(pid, frame.midX - window.minX, frame.midY - window.minY)
    let (down, up, _, button) = buttonTypes(rest.count > 4 ? rest[4] : "left")
    post(CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: at, mouseButton: button))
    post(CGEvent(mouseEventSource: source, mouseType: down, mouseCursorPosition: at, mouseButton: button))
    post(CGEvent(mouseEventSource: source, mouseType: up, mouseCursorPosition: at, mouseButton: button))
    print(found[index].line(relativeTo: window))
  case "selection":
    guard let area = webArea(pid) else { fail("no AXWebArea") }
    guard let range = attribute(area, "AXSelectedTextMarkerRange") else { print("{\"selection\":null}"); return }
    var value: CFTypeRef?
    let status = AXUIElementCopyParameterizedAttributeValue(area, "AXStringForTextMarkerRange" as CFString, range, &value)
    let selected = status == .success ? (value as? String ?? "") : ""
    print("{\"selection\":\(json(redact(selected))),\"length\":\(selected.count)}")
  case "attr":
    guard rest.count >= 4 else { fail("ax attr <role|*> <text> <attribute> [index]") }
    let found = matches(pid, role: rest[1], needle: rest[2])
    let index = rest.count > 4 ? Int(rest[4]) ?? 0 : 0
    guard index < found.count else { fail("ax attr: \(found.count) match(es); index \(index) unavailable") }
    print(describe(attribute(found[index].element, rest[3]), window: window))
  case "bounds":
    guard rest.count >= 5, let location = Int(rest[3]), let length = Int(rest[4]) else {
      fail("ax bounds <role|*> <text> <location> <length> [index]")
    }
    let found = matches(pid, role: rest[1], needle: rest[2])
    let index = rest.count > 5 ? Int(rest[5]) ?? 0 : 0
    guard index < found.count else { fail("ax bounds: \(found.count) match(es); index \(index) unavailable") }
    var range = CFRange(location: location, length: length)
    guard let parameter = AXValueCreate(.cfRange, &range) else { fail("AX range") }
    var value: CFTypeRef?
    let status = AXUIElementCopyParameterizedAttributeValue(found[index].element, kAXBoundsForRangeParameterizedAttribute as CFString, parameter, &value)
    guard status == .success else { fail("ax bounds: AXBoundsForRange failed (\(status.rawValue))") }
    print(describe(value, window: window))
  case "focuspath":
    let app = AXUIElementCreateApplication(pid)
    guard let value = attribute(app, kAXFocusedUIElementAttribute as String),
          CFGetTypeID(value) == AXUIElementGetTypeID() else { print("{\"focused\":null}"); return }
    var element: AXUIElement? = (value as! AXUIElement)
    var depth = 0
    while let current = element, depth < 40 {
      let item = node(current, depth: depth)
      print(item.line(relativeTo: window))
      if item.role == "AXWebArea" || item.role == "AXWindow" { break }
      element = attribute(current, kAXParentAttribute as String).flatMap {
        CFGetTypeID($0) == AXUIElementGetTypeID() ? ($0 as! AXUIElement) : nil
      }
      depth += 1
    }
  default:
    fail("unknown ax subcommand \(sub)")
  }
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
if args.first == "screen" {
  guard let screen = NSScreen.main else { fail("no main screen") }
  let f = screen.frame, v = screen.visibleFrame
  print("{\"frame\":[\(f.minX),\(f.minY),\(f.width),\(f.height)],\"visible\":[\(v.minX),\(v.minY),\(v.width),\(v.height)],\"scale\":\(screen.backingScaleFactor)}")
  exit(0)
}
if args.first == "idle" {
  print(Int(idleSeconds()))
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
case "ctrlclick":
  ensureFront(pid)
  let at = point(pid, number(0), number(1))
  for type in [CGEventType.leftMouseDown, .leftMouseUp] {
    let event = CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: at, mouseButton: .left)
    event?.flags = .maskControl
    post(event)
  }
case "drag":
  ensureFront(pid)
  let from = point(pid, number(0), number(1))
  let to = point(pid, number(2), number(3))
  let steps = rest.count > 4 ? max(1, Int(rest[4]) ?? 12) : 12
  let hold = rest.count > 5 ? UInt32(rest[5]) ?? 0 : 0
  post(CGEvent(mouseEventSource: source, mouseType: .mouseMoved, mouseCursorPosition: from, mouseButton: .left))
  post(CGEvent(mouseEventSource: source, mouseType: .leftMouseDown, mouseCursorPosition: from, mouseButton: .left))
  for step in 1...steps {
    let t = Double(step) / Double(steps)
    let at = CGPoint(x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t)
    post(CGEvent(mouseEventSource: source, mouseType: .leftMouseDragged, mouseCursorPosition: at, mouseButton: .left))
    usleep(16_000)
  }
  if hold > 0 { usleep(hold * 1000) }
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
case "type":
  ensureFront(pid)
  guard let string = rest.first else { fail("type: missing text") }
  for character in string.utf16 {
    var unit = character
    for down in [true, false] {
      let event = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: down)
      event?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &unit)
      post(event)
    }
  }
case "ax":
  axCommand(pid, rest)
default:
  fail("unknown command \(command)")
}
