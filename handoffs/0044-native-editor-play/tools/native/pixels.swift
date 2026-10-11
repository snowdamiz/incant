// pixels: reads measurements from the 0044 native window captures (PNG, device pixels).
//
// Used to measure what the screenshots show (traffic-light and logo centres, the
// viewport hole's edges, focus rings) instead of estimating by eye. Reads files only.
//
// Build: swiftc -O pixels.swift -o /tmp/incant0044/pixels
// Usage: pixels <png> size
//        pixels <png> at <x> <y>                         -> #rrggbbaa
//        pixels <png> row <y> <x0> <x1> [tolerance]      -> colour runs along a row
//        pixels <png> col <x> <y0> <y1> [tolerance]      -> colour runs down a column
//        pixels <png> bbox <x0> <y0> <x1> <y1> <#rrggbb> <tolerance>
//                 -> bounding box and centre of pixels that differ from the background
//        pixels <png> crop <x> <y> <w> <h> <out.png> [scale]  -> nearest-neighbour enlargement
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

func fail(_ message: String) -> Never {
  FileHandle.standardError.write((message + "\n").data(using: .utf8)!)
  exit(1)
}

struct Bitmap {
  let width: Int
  let height: Int
  var bytes: [UInt8]

  init(path: String) {
    guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { fail("cannot read \(path)") }
    width = image.width
    height = image.height
    bytes = [UInt8](repeating: 0, count: width * height * 4)
    let space = CGColorSpace(name: CGColorSpace.sRGB)!
    bytes.withUnsafeMutableBytes { buffer in
      guard let context = CGContext(data: buffer.baseAddress, width: width, height: height, bitsPerComponent: 8,
                                    bytesPerRow: width * 4, space: space,
                                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { fail("no context") }
      context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
    }
  }

  func pixel(_ x: Int, _ y: Int) -> [Int] {
    guard x >= 0, y >= 0, x < width, y < height else { fail("(\(x), \(y)) is outside \(width)x\(height)") }
    let i = (y * width + x) * 4
    return [Int(bytes[i]), Int(bytes[i + 1]), Int(bytes[i + 2]), Int(bytes[i + 3])]
  }
}

func hex(_ p: [Int]) -> String { String(format: "#%02x%02x%02x%02x", p[0], p[1], p[2], p[3]) }

func parseHex(_ text: String) -> [Int] {
  let digits = text.hasPrefix("#") ? String(text.dropFirst()) : text
  guard digits.count == 6, let value = Int(digits, radix: 16) else { fail("colour must be #rrggbb") }
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255]
}

func distance(_ a: [Int], _ b: [Int]) -> Int { (0..<3).map { abs(a[$0] - b[$0]) }.max()! }

/// Runs of similar colour along a line of pixels.
func runs(_ points: [(Int, Int)], _ bitmap: Bitmap, _ tolerance: Int, along: (Int, Int) -> Int) {
  var start = 0
  for index in 1...points.count {
    let ended = index == points.count || distance(bitmap.pixel(points[index].0, points[index].1),
                                                    bitmap.pixel(points[start].0, points[start].1)) > tolerance
    if ended {
      let first = points[start], last = points[index - 1]
      print("\(along(first.0, first.1))-\(along(last.0, last.1)) (\(index - start)) \(hex(bitmap.pixel(first.0, first.1)))")
      start = index
    }
  }
}

let args = Array(CommandLine.arguments.dropFirst())
guard args.count >= 2 else { fail("usage: pixels <png> <command> ...") }
let bitmap = Bitmap(path: args[0])
func int(_ index: Int) -> Int {
  guard index < args.count, let value = Int(args[index]) else { fail("\(args[1]): missing integer argument \(index - 1)") }
  return value
}

switch args[1] {
case "size":
  print("\(bitmap.width) \(bitmap.height)")
case "at":
  print(hex(bitmap.pixel(int(2), int(3))))
case "row":
  let y = int(2)
  runs((int(3)...int(4)).map { ($0, y) }, bitmap, args.count > 5 ? int(5) : 0) { x, _ in x }
case "col":
  let x = int(2)
  runs((int(3)...int(4)).map { (x, $0) }, bitmap, args.count > 5 ? int(5) : 0) { _, y in y }
case "bbox":
  guard args.count > 7 else { fail("bbox: <x0> <y0> <x1> <y1> <#rrggbb> <tolerance>") }
  let background = parseHex(args[6])
  let tolerance = int(7)
  var box: (Int, Int, Int, Int)? = nil
  for y in int(3)...int(5) {
    for x in int(2)...int(4) where distance(bitmap.pixel(x, y), background) > tolerance {
      box = box.map { (min($0.0, x), min($0.1, y), max($0.2, x), max($0.3, y)) } ?? (x, y, x, y)
    }
  }
  guard let found = box else { print("none"); exit(0) }
  print("x \(found.0)-\(found.2) y \(found.1)-\(found.3) centre \(Double(found.0 + found.2) / 2) \(Double(found.1 + found.3) / 2)")
case "crop":
  let (x0, y0, w, h) = (int(2), int(3), int(4), int(5))
  guard args.count > 6 else { fail("crop: missing output path") }
  let scale = args.count > 7 ? int(7) : 1
  var out = [UInt8](repeating: 0, count: w * scale * h * scale * 4)
  for y in 0..<(h * scale) {
    for x in 0..<(w * scale) {
      let p = bitmap.pixel(x0 + x / scale, y0 + y / scale)
      let i = (y * w * scale + x) * 4
      for c in 0..<4 { out[i + c] = UInt8(p[c]) }
    }
  }
  let space = CGColorSpace(name: CGColorSpace.sRGB)!
  let image: CGImage = out.withUnsafeMutableBytes { buffer in
    let context = CGContext(data: buffer.baseAddress, width: w * scale, height: h * scale, bitsPerComponent: 8,
                            bytesPerRow: w * scale * 4, space: space,
                            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    return context.makeImage()!
  }
  guard let destination = CGImageDestinationCreateWithURL(URL(fileURLWithPath: args[6]) as CFURL,
                                                          UTType.png.identifier as CFString, 1, nil) else { fail("cannot write") }
  CGImageDestinationAddImage(destination, image, nil)
  guard CGImageDestinationFinalize(destination) else { fail("cannot write \(args[6])") }
  print("\(args[6]) \(w * scale)x\(h * scale)")
default:
  fail("unknown command \(args[1])")
}
