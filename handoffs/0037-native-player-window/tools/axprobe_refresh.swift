// Delta review helper for handoff 0037 (Claude). Reads the game window's
// title-bar buttons, its full-screen attribute and the application menu, and
// can press one title-bar button. It acts only on the exact PID whose executable
// resolves to $GAME_BINARY and never matches processes by name.
//
//   axprobe buttons PID
//   axprobe menu PID
//   axprobe press PID zoom|fullscreen|close
import ApplicationServices
import Foundation

let gameTitle = "Incant native game"

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data(("axprobe: " + message + "\n").utf8))
    exit(2)
}

func emit(_ fields: [String: Any]) {
    var record = fields
    record["epoch"] = Date().timeIntervalSince1970
    let data = try! JSONSerialization.data(withJSONObject: record, options: [.sortedKeys])
    print(String(decoding: data, as: UTF8.self))
}

func verifiedPid(_ text: String) -> pid_t {
    guard let pid = pid_t(text), pid > 1 else { fail("invalid pid \(text)") }
    guard let expected = ProcessInfo.processInfo.environment["GAME_BINARY"] else { fail("GAME_BINARY is not set") }
    var buffer = [CChar](repeating: 0, count: 4096)
    guard proc_pidpath(pid, &buffer, UInt32(buffer.count)) > 0 else { fail("pid \(pid) is not running") }
    let actual = String(cString: buffer)
    let resolved = URL(fileURLWithPath: expected).resolvingSymlinksInPath().path
    guard actual == resolved else { fail("pid \(pid) runs \(actual), not \(resolved)") }
    return pid
}

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}

func settable(_ element: AXUIElement, _ name: String) -> Any {
    var flag = DarwinBoolean(false)
    let error = AXUIElementIsAttributeSettable(element, name as CFString, &flag)
    return error == .success ? flag.boolValue : "AXError \(error.rawValue)"
}

func gameWindow(_ pid: pid_t) -> AXUIElement {
    let windows = attribute(AXUIElementCreateApplication(pid), kAXWindowsAttribute) as? [AXUIElement] ?? []
    let matches = windows.filter { attribute($0, kAXTitleAttribute) as? String == gameTitle }
    guard matches.count == 1 else { fail("expected one '\(gameTitle)' window, found \(matches.count)") }
    return matches[0]
}

let buttonAttributes = ["zoom": "AXZoomButton", "fullscreen": "AXFullScreenButton", "close": "AXCloseButton", "minimize": "AXMinimizeButton"]

func button(_ window: AXUIElement, _ name: String) -> AXUIElement? {
    guard let key = buttonAttributes[name], let value = attribute(window, key) else { return nil }
    return (value as! AXUIElement)
}

let arguments = Array(CommandLine.arguments.dropFirst())
guard arguments.count >= 2 else { fail("usage: axprobe buttons|menu|press PID [button]") }
let pid = verifiedPid(arguments[1])
switch arguments[0] {
case "buttons":
    let window = gameWindow(pid)
    var fields: [String: Any] = ["pid": Int(pid)]
    for name in buttonAttributes.keys.sorted() {
        if let element = button(window, name) {
            let state: [String: Any] = [
                "subrole": attribute(element, kAXSubroleAttribute) as? String ?? "",
                "enabled": attribute(element, kAXEnabledAttribute) as? Bool ?? false,
            ]
            fields[name] = state
        } else {
            fields[name] = NSNull()
        }
    }
    fields["full_screen"] = attribute(window, "AXFullScreen") as? Bool ?? NSNull()
    fields["full_screen_settable"] = settable(window, "AXFullScreen")
    fields["size_settable"] = settable(window, kAXSizeAttribute)
    emit(fields)
case "menu":
    let app = AXUIElementCreateApplication(pid)
    guard let bar = attribute(app, kAXMenuBarAttribute) else { fail("no menu bar") }
    var menus: [[String: Any]] = []
    for item in attribute(bar as! AXUIElement, kAXChildrenAttribute) as? [AXUIElement] ?? [] {
        var entries: [[String: Any]] = []
        for menu in attribute(item, kAXChildrenAttribute) as? [AXUIElement] ?? [] {
            for entry in attribute(menu, kAXChildrenAttribute) as? [AXUIElement] ?? [] {
                entries.append([
                    "title": attribute(entry, kAXTitleAttribute) as? String ?? "",
                    "enabled": attribute(entry, kAXEnabledAttribute) as? Bool ?? NSNull(),
                    "cmd_char": attribute(entry, "AXMenuItemCmdChar") as? String ?? NSNull(),
                    "cmd_modifiers": attribute(entry, "AXMenuItemCmdModifiers") as? Int ?? NSNull(),
                ])
            }
        }
        menus.append(["title": attribute(item, kAXTitleAttribute) as? String ?? "", "items": entries])
    }
    emit(["pid": Int(pid), "menus": menus])
case "press":
    guard arguments.count == 3, let element = button(gameWindow(pid), arguments[2]) else { fail("no such button") }
    let enabled = attribute(element, kAXEnabledAttribute) as? Bool ?? false
    let error = AXUIElementPerformAction(element, kAXPressAction as CFString)
    emit(["pid": Int(pid), "pressed": arguments[2], "enabled_before": enabled, "ax_error": Int(error.rawValue)])
default:
    fail("unknown command \(arguments[0])")
}
