// imgtool: local edits and measurements of the 0050 window captures. Pixel coordinates, top-left
// origin. Reads and writes PNG only; never captures anything itself. Pixels stay in the image's own
// colour space (the captures are Display P3), so a crop or redaction changes no other value.
//
// Build: swiftc -O imgtool.swift -o /tmp/incant0050/imgtool
// Usage: imgtool crop <in> <out> <x> <y> <w> <h>
//        imgtool redact <in> <out> <x> <y> <w> <h> <rrggbb>   -> fills that rectangle
//        imgtool stack <out> <gap> <rrggbb> <in>...           -> one column, left-aligned
//        imgtool sample <in> <x> <y>                          -> the pixel's rrggbb
//        imgtool diff <a> <b>                                 -> how many pixels differ, and where
//        imgtool ink <in> <x> <y> <w> <h> <rrggbb> <tolerance>
//            -> runs of columns holding any pixel that differs from the background by more than
//               the tolerance (max channel difference), each with its rows: glyphs, roughly
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
default:
  fail("usage: imgtool crop|redact|stack|sample|ink ...")
}
