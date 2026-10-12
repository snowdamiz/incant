// probe: loads a page in an offscreen system WKWebView (never shown) and prints, for every
// `input.model` on it, its client width and the width its text needs (scrollWidth), after the
// fonts load. scrollWidth > clientWidth means WebKit ellipsizes the text (handoff 0048).
// Build: swiftc -O probe.swift -o /tmp/incant0048/webkit-probe
// Run:   webkit-probe <directory with the page and font> <page.html>
import AppKit
import WebKit

final class Probe: NSObject, WKNavigationDelegate {
  let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 400, height: 300))
  init(directory: URL, page: String) {
    super.init()
    view.navigationDelegate = self
    view.loadFileURL(directory.appendingPathComponent(page), allowingReadAccessTo: directory)
  }
  func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
    let script = """
      await document.fonts.ready;
      const fields = [...document.querySelectorAll('input.model')].map((el) => ({
        id: el.id || null, n: el.value.length, width: el.style.width,
        clientWidth: el.clientWidth, scrollWidth: el.scrollWidth, overflows: el.scrollWidth > el.clientWidth }));
      return JSON.stringify({
        fontLoaded: document.fonts.check("12px 'JetBrains Mono Variable'"),
        textWidth: document.getElementById('text').getBoundingClientRect().width,
        userAgent: navigator.userAgent, fields });
      """
    webView.callAsyncJavaScript(script, arguments: [:], in: nil, in: .page) { result in
      switch result {
      case .success(let value): print(value as? String ?? "\(value)")
      case .failure(let error): print("error: \(error)")
      }
      NSApp.terminate(nil)
    }
  }
}

let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
let probe = Probe(directory: URL(fileURLWithPath: CommandLine.arguments[1]), page: CommandLine.arguments[2])
DispatchQueue.main.asyncAfter(deadline: .now() + 20) { print("timeout"); NSApp.terminate(nil) }
app.run()
