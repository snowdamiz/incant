// Compiled with -import-objc-header incant_game.h; this is the actual C import.
import Foundation

if CommandLine.arguments.count != 2 { exit(64) }
let status: Int32
if CommandLine.arguments[1] == "--null" {
    status = incant_game_run_v1(nil, 1)
} else {
    do {
        let file = try FileHandle(forReadingFrom: URL(fileURLWithPath: CommandLine.arguments[1]))
        defer { try? file.close() }
        let bytes = try file.read(upToCount: 65537) ?? Data()
        status = bytes.withUnsafeBytes { allocation in
            incant_game_run_v1(allocation.bindMemory(to: UInt8.self).baseAddress, bytes.count)
        }
    } catch { exit(65) }
}
print("{\"abi_status\":\(status)}")
exit(status)
