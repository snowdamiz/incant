import UIKit
import QuartzCore

// This is a native surface/lifecycle adapter, with no authored game UI. The
// cooked game's retained renderer owns every displayed widget and scene pixel.
@MainActor
final class GameView: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }
    private struct Gesture {
        let handle: UInt64
        let epoch: UInt64
        let slot: UInt32
    }
    private var handle: UInt64 = 0
    private var gestures: [ObjectIdentifier: Gesture] = [:]
    private var link: CADisplayLink?
    private var wantsActive = false
    private var terminal = false
    private var geometry: CGSize = .zero
    private var scale: CGFloat = 0
    var onFailure: ((String) -> Void)?

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
    }
    required init?(coder: NSCoder) { fatalError("Only native construction is supported") }
    isolated deinit { close() }

    func setActive(_ active: Bool) {
        wantsActive = active
        if !active {
            link?.invalidate()
            link = nil
            gestures.removeAll()
        }
        if handle != 0 {
            guard accept(incant_apple_active_v1(handle, active ? 1 : 0)) else { return }
        }
        if active { configure() }
    }
    func close() {
        link?.invalidate()
        link = nil
        gestures.removeAll()
        if handle != 0 {
            let original = handle
            handle = 0
            _ = incant_apple_close_v1(original)
        }
    }
    private func fail(_ message: String) {
        terminal = true
        wantsActive = false
        close()
        onFailure?(message)
    }
    @discardableResult
    private func accept(_ status: Int32, staleTouchAllowed: Bool = false) -> Bool {
        if status == 0 || (staleTouchAllowed && status == 3) { return true }
        var bytes = [UInt8](repeating: 0, count: 4096)
        let count = incant_apple_error_v1(&bytes, bytes.count)
        let message = String(decoding: bytes.prefix(count), as: UTF8.self)
        fail("Native host status \(status): \(message)")
        return false
    }
    override func layoutSubviews() {
        super.layoutSubviews()
        configure()
    }
    private func configure() {
        guard !terminal, let window else { return }
        let newScale = window.screen.scale
        let size = bounds.size
        guard size.width.isFinite, size.height.isFinite, newScale.isFinite,
              newScale > 0, size.width > 0, size.height > 0 else {
            if handle != 0 { _ = accept(incant_apple_active_v1(handle, 0)) }
            return
        }
        let pixels = CGSize(width: (size.width * newScale).rounded(), height: (size.height * newScale).rounded())
        guard pixels.width >= 16, pixels.height >= 16, pixels.width <= 4096,
              pixels.height <= 4096, pixels.width * pixels.height <= 4 * 1024 * 1024 else {
            fail("Native surface exceeds the bounded output size")
            return
        }
        let metal = layer as! CAMetalLayer
        if geometry != size || scale != newScale {
            // Retire native input before publishing replacement hit coordinates.
            if handle != 0 {
                guard accept(incant_apple_active_v1(handle, 0)) else { return }
                gestures.removeAll()
            }
            metal.contentsScale = newScale
            metal.drawableSize = pixels
            if handle != 0 {
                guard accept(incant_apple_resize_v1(handle, UInt32(pixels.width), UInt32(pixels.height), Float(size.width), Float(size.height))) else { return }
            }
            geometry = size
            scale = newScale
        }
        if handle == 0 {
            guard let data = Bundle.main.resourceURL?.appendingPathComponent("data") else {
                fail("Missing bundled cooked game data")
                return
            }
            let path = Array(data.path.utf8)
            var opened: UInt64 = 0
            let status = path.withUnsafeBufferPointer {
                incant_apple_open_v1(Unmanaged.passUnretained(metal).toOpaque(), $0.baseAddress, $0.count,
                    UInt32(pixels.width), UInt32(pixels.height), Float(size.width), Float(size.height), &opened)
            }
            guard accept(status) else { return }
            handle = opened
        }
        guard accept(incant_apple_active_v1(handle, wantsActive ? 1 : 0)) else { return }
        if wantsActive && link == nil {
            let proxy = FrameProxy(self)
            let next = CADisplayLink(target: proxy, selector: #selector(FrameProxy.frame))
            next.add(to: .main, forMode: .common)
            link = next
        }
    }
    fileprivate func frame() {
        guard wantsActive, handle != 0, !terminal else { return }
        _ = accept(incant_apple_frame_v1(handle))
    }
    override func willMove(toWindow newWindow: UIWindow?) {
        if newWindow == nil { close() }
        super.willMove(toWindow: newWindow)
    }
    private func touch(_ touches: Set<UITouch>, phase: UInt32) {
        guard handle != 0, !terminal else { return }
        for touch in touches {
            let id = ObjectIdentifier(touch)
            if phase == 0 {
                let epoch = incant_apple_epoch_v1(handle)
                guard epoch != 0 else { continue }
                let occupied = Set(gestures.values.map(\.slot))
                guard let slot = (UInt32(0)..<16).first(where: { !occupied.contains($0) }) else {
                    fail("Native touch capacity exceeded")
                    return
                }
                gestures[id] = Gesture(handle: handle, epoch: epoch, slot: slot)
            }
            guard let gesture = gestures[id] else { continue }
            let position = touch.location(in: self)
            let status = incant_apple_touch_v1(gesture.handle, gesture.epoch, gesture.slot, phase, Float(position.x), Float(position.y))
            if phase >= 2 || status == 3 { gestures.removeValue(forKey: id) }
            guard accept(status, staleTouchAllowed: true) else { return }
        }
    }
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) { touch(touches, phase: 0) }
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) { touch(touches, phase: 1) }
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) { touch(touches, phase: 2) }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { touch(touches, phase: 3) }
}

@MainActor
private final class FrameProxy: NSObject {
    weak var view: GameView?
    init(_ view: GameView) { self.view = view }
    @objc func frame() { view?.frame() }
}
