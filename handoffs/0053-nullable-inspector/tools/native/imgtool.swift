// imgtool: local edits and measurements of the 0053 window captures (copied unchanged in behaviour
// from 0052: 0050's tool plus `rdiff` and `blob`). Pixel coordinates, top-left origin. Reads and
// writes PNG only; never
// captures anything itself. Pixels stay in the image's own colour space (the captures are
// Display P3), so a crop or redaction changes no other value.
//
// Build: swiftc -O imgtool.swift -o /tmp/incant0053/imgtool
// Usage: imgtool crop <in> <out> <x> <y> <w> <h>
//        imgtool redact <in> <out> <x> <y> <w> <h> <rrggbb>   -> fills that rectangle
//        imgtool stack <out> <gap> <rrggbb> <in>...           -> one column, left-aligned
//        imgtool sample <in> <x> <y>                          -> the pixel's rrggbb
//        imgtool diff <a> <b>                                 -> how many pixels differ, and where
//        imgtool ink <in> <x> <y> <w> <h> <rrggbb> <tolerance>
//            -> runs of columns holding any pixel that differs from the background by more than
//               the tolerance (max channel difference), each with its rows: glyphs, roughly
//        imgtool rdiff <a> <b> <x> <y> <w> <h> [tolerance]
//            -> pixels inside the rectangle whose largest channel difference exceeds the
//               tolerance (default 0), their box, and the largest difference seen
//        imgtool blob <in> <x> <y> <w> <h> <tolerance> [rows|rrggbb]
//            -> the pixels inside the rectangle that differ from the background by more than the
//               tolerance: count, centroid, box, mean colour and three extreme points (the
//               farthest pixel from the centroid, the farthest from that, and the one farthest
//               from the line through both: a triangle's corners). "rows" (the default) takes
//               each row's per-channel median inside the rectangle as that row's background, so
//               a vertical gradient behind a small shape is not counted as the shape.
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

func fail(_ message: String) -> Never {
  FileHandle.standardError.write((message + "\n").data(using: .utf8)!)
  exit(1)
}

struct Bitmap {
  var pixels: [UInt8]
  let width: Int
  let height: Int
  let space: CGColorSpace

  init(path: String) {
    guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { fail("cannot read \(path)") }
    width = image.width
    height = image.height
    pixels = [UInt8](repeating: 0, count: width * height * 4)
    let space = image.colorSpace ?? CGColorSpace(name: CGColorSpace.sRGB)!
    self.space = space
    pixels.withUnsafeMutableBytes { buffer in
      let context = CGContext(data: buffer.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                              space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
      context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
    }
  }

  init(width: Int, height: Int, fill: (UInt8, UInt8, UInt8), space: CGColorSpace) {
    self.width = width
    self.height = height
    self.space = space
    pixels = [UInt8](repeating: 255, count: width * height * 4)
    for i in stride(from: 0, to: pixels.count, by: 4) {
      pixels[i] = fill.0; pixels[i + 1] = fill.1; pixels[i + 2] = fill.2
    }
  }

  func rgb(_ x: Int, _ y: Int) -> (UInt8, UInt8, UInt8) {
    let i = (y * width + x) * 4
    return (pixels[i], pixels[i + 1], pixels[i + 2])
  }

  mutating func set(_ x: Int, _ y: Int, _ c: (UInt8, UInt8, UInt8)) {
    let i = (y * width + x) * 4
    pixels[i] = c.0; pixels[i + 1] = c.1; pixels[i + 2] = c.2; pixels[i + 3] = 255
  }

  func save(_ path: String) {
    var copy = pixels
    let image: CGImage = copy.withUnsafeMutableBytes { buffer in
      let context = CGContext(data: buffer.baseAddress, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
                              space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
      return context.makeImage()!
    }
    guard let destination = CGImageDestinationCreateWithURL(URL(fileURLWithPath: path) as CFURL, UTType.png.identifier as CFString, 1, nil) else {
      fail("cannot write \(path)")
    }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else { fail("cannot write \(path)") }
  }
}

func color(_ hex: String) -> (UInt8, UInt8, UInt8) {
  guard hex.count == 6, let value = UInt32(hex, radix: 16) else { fail("colour must be rrggbb") }
  return (UInt8(value >> 16 & 255), UInt8(value >> 8 & 255), UInt8(value & 255))
}

func hex(_ c: (UInt8, UInt8, UInt8)) -> String { String(format: "%02x%02x%02x", c.0, c.1, c.2) }

let args = Array(CommandLine.arguments.dropFirst())
func int(_ index: Int) -> Int {
  guard index < args.count, let value = Int(args[index]) else { fail("\(args.first ?? ""): missing integer \(index)") }
  return value
}
func rect(_ image: Bitmap, from index: Int) -> (Int, Int, Int, Int) {
  let (x, y, w, h) = (int(index), int(index + 1), int(index + 2), int(index + 3))
  guard x >= 0, y >= 0, w > 0, h > 0, x + w <= image.width, y + h <= image.height else {
    fail("rectangle \(x),\(y) \(w)x\(h) is outside the \(image.width)x\(image.height) image")
  }
  return (x, y, w, h)
}

switch args.first {
case "crop":
  let image = Bitmap(path: args[1])
  let (x, y, w, h) = rect(image, from: 3)
  var out = Bitmap(width: w, height: h, fill: (0, 0, 0), space: image.space)
  for row in 0..<h { for column in 0..<w { out.set(column, row, image.rgb(x + column, y + row)) } }
  out.save(args[2])
case "redact":
  var image = Bitmap(path: args[1])
  let (x, y, w, h) = rect(image, from: 3)
  let fill = color(args[7])
  for row in y..<(y + h) { for column in x..<(x + w) { image.set(column, row, fill) } }
  image.save(args[2])
case "stack":
  let gap = int(2)
  let fill = color(args[3])
  let parts = args.dropFirst(4).map { Bitmap(path: $0) }
  guard !parts.isEmpty else { fail("stack: no inputs") }
  let width = parts.map(\.width).max()!
  let height = parts.map(\.height).reduce(0, +) + gap * (parts.count - 1)
  var out = Bitmap(width: width, height: height, fill: fill, space: parts[0].space)
  var top = 0
  for part in parts {
    for row in 0..<part.height { for column in 0..<part.width { out.set(column, top + row, part.rgb(column, row)) } }
    top += part.height + gap
  }
  out.save(args[1])
case "sample":
  let image = Bitmap(path: args[1])
  let (x, y) = (int(2), int(3))
  guard x < image.width, y < image.height else { fail("outside the image") }
  print(hex(image.rgb(x, y)))
case "diff":
  let a = Bitmap(path: args[1])
  let b = Bitmap(path: args[2])
  guard a.width == b.width, a.height == b.height else { fail("sizes differ: \(a.width)x\(a.height) and \(b.width)x\(b.height)") }
  var count = 0
  var box = (minX: Int.max, minY: Int.max, maxX: -1, maxY: -1)
  for y in 0..<a.height {
    for x in 0..<a.width where a.rgb(x, y) != b.rgb(x, y) {
      count += 1
      box = (min(box.minX, x), min(box.minY, y), max(box.maxX, x), max(box.maxY, y))
    }
  }
  let spaces = "\"\(a.space.name.map { $0 as String } ?? "?")\", \"\(b.space.name.map { $0 as String } ?? "?")\""
  print(count == 0 ? "{\"differing\":0,\"spaces\":[\(spaces)]}" :
        "{\"differing\":\(count),\"box\":[\(box.minX),\(box.minY),\(box.maxX - box.minX + 1),\(box.maxY - box.minY + 1)],\"spaces\":[\(spaces)]}")
case "ink":
  let image = Bitmap(path: args[1])
  let (x, y, w, h) = rect(image, from: 2)
  let background = color(args[6])
  let tolerance = int(7)
  func inked(_ column: Int, _ row: Int) -> Bool {
    let c = image.rgb(column, row)
    return max(abs(Int(c.0) - Int(background.0)), abs(Int(c.1) - Int(background.1)), abs(Int(c.2) - Int(background.2))) > tolerance
  }
  var runs: [(start: Int, end: Int, top: Int, bottom: Int)] = []
  var current: (start: Int, end: Int, top: Int, bottom: Int)?
  for column in x..<(x + w) {
    let rows = (y..<(y + h)).filter { inked(column, $0) }
    if let first = rows.first, let last = rows.last {
      if let open = current {
        current = (open.start, column, min(open.top, first), max(open.bottom, last))
      } else {
        current = (column, column, first, last)
      }
    } else if let open = current {
      runs.append(open)
      current = nil
    }
  }
  if let open = current { runs.append(open) }
  print("{\"runs\":\(runs.count),\"first_x\":\(runs.first?.start ?? -1),\"last_x\":\(runs.last?.end ?? -1),\"glyphs\":[" +
        runs.map { "[\($0.start),\($0.end),\($0.top),\($0.bottom)]" }.joined(separator: ",") + "]}")
case "rdiff":
  let a = Bitmap(path: args[1])
  let b = Bitmap(path: args[2])
  guard a.width == b.width, a.height == b.height else { fail("sizes differ: \(a.width)x\(a.height) and \(b.width)x\(b.height)") }
  let (x, y, w, h) = rect(a, from: 3)
  let tolerance = args.count > 7 ? int(7) : 0
  var count = 0
  var largest = 0
  var box = (minX: Int.max, minY: Int.max, maxX: -1, maxY: -1)
  for row in y..<(y + h) {
    for column in x..<(x + w) {
      let p = a.rgb(column, row), q = b.rgb(column, row)
      let d = max(abs(Int(p.0) - Int(q.0)), abs(Int(p.1) - Int(q.1)), abs(Int(p.2) - Int(q.2)))
      largest = max(largest, d)
      if d > tolerance {
        count += 1
        box = (min(box.minX, column), min(box.minY, row), max(box.maxX, column), max(box.maxY, row))
      }
    }
  }
  print(count == 0 ? "{\"differing\":0,\"largest\":\(largest),\"tolerance\":\(tolerance),\"pixels\":\(w * h)}" :
        "{\"differing\":\(count),\"largest\":\(largest),\"tolerance\":\(tolerance),\"pixels\":\(w * h),\"box\":[\(box.minX),\(box.minY),\(box.maxX - box.minX + 1),\(box.maxY - box.minY + 1)]}")
case "blob":
  let image = Bitmap(path: args[1])
  let (x, y, w, h) = rect(image, from: 2)
  let tolerance = int(6)
  let mode = args.count > 7 ? args[7] : "rows"
  let fixed: (UInt8, UInt8, UInt8)? = mode == "rows" ? nil : color(mode)
  func median(_ values: inout [UInt8]) -> UInt8 {
    values.sort()
    return values[values.count / 2]
  }
  var points: [(Int, Int)] = []
  var sum = (0, 0, 0)
  var backgrounds: [String: Int] = [:]
  for row in y..<(y + h) {
    var background = fixed ?? (0, 0, 0)
    if fixed == nil {
      var r = [UInt8](), g = [UInt8](), b = [UInt8]()
      r.reserveCapacity(w); g.reserveCapacity(w); b.reserveCapacity(w)
      for column in x..<(x + w) {
        let c = image.rgb(column, row)
        r.append(c.0); g.append(c.1); b.append(c.2)
      }
      background = (median(&r), median(&g), median(&b))
    }
    backgrounds[hex(background), default: 0] += 1
    for column in x..<(x + w) {
      let c = image.rgb(column, row)
      let d = max(abs(Int(c.0) - Int(background.0)), abs(Int(c.1) - Int(background.1)), abs(Int(c.2) - Int(background.2)))
      if d > tolerance {
        points.append((column, row))
        sum = (sum.0 + Int(c.0), sum.1 + Int(c.1), sum.2 + Int(c.2))
      }
    }
  }
  let common = backgrounds.sorted { $0.value > $1.value }.prefix(4).map { "[\"\($0.key)\",\($0.value)]" }.joined(separator: ",")
  guard !points.isEmpty else {
    print("{\"count\":0,\"backgrounds\":[\(common)]}")
    exit(0)
  }
  let n = Double(points.count)
  let cx = points.reduce(0.0) { $0 + Double($1.0) } / n
  let cy = points.reduce(0.0) { $0 + Double($1.1) } / n
  var mu = (xx: 0.0, yy: 0.0, xy: 0.0)
  var box = (minX: Int.max, minY: Int.max, maxX: -1, maxY: -1)
  for (px, py) in points {
    let dx = Double(px) - cx, dy = Double(py) - cy
    mu = (mu.xx + dx * dx, mu.yy + dy * dy, mu.xy + dx * dy)
    box = (min(box.minX, px), min(box.minY, py), max(box.maxX, px), max(box.maxY, py))
  }
  func farthest(from p: (Double, Double)) -> (Int, Int) {
    points.max { a, b in
      let da = pow(Double(a.0) - p.0, 2) + pow(Double(a.1) - p.1, 2)
      let db = pow(Double(b.0) - p.0, 2) + pow(Double(b.1) - p.1, 2)
      return da < db
    }!
  }
  let first = farthest(from: (cx, cy))
  let second = farthest(from: (Double(first.0), Double(first.1)))
  let third = points.max { a, b in
    func distance(_ p: (Int, Int)) -> Double {
      abs(Double(second.0 - first.0) * Double(first.1 - p.1) - Double(first.0 - p.0) * Double(second.1 - first.1))
    }
    return distance(a) < distance(b)
  }!
  let mean = (UInt8(sum.0 / points.count), UInt8(sum.1 / points.count), UInt8(sum.2 / points.count))
  print(String(format: "{\"count\":%d,\"centroid\":[%.3f,%.3f],\"box\":[%d,%d,%d,%d],\"moments\":[%.3f,%.3f,%.3f],", points.count, cx, cy,
               box.minX, box.minY, box.maxX - box.minX + 1, box.maxY - box.minY + 1, mu.xx / n, mu.yy / n, mu.xy / n) +
        "\"vertices\":[[\(first.0),\(first.1)],[\(second.0),\(second.1)],[\(third.0),\(third.1)]],\"mean\":\"\(hex(mean))\",\"backgrounds\":[\(common)]}")
default:
  fail("usage: imgtool crop|redact|stack|sample|diff|ink|rdiff|blob ...")
}
