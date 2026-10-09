import UIKit

@_silgen_name("incant_smoke_run") func incantSmokeRun() -> Int32

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, configurationForConnecting session: UISceneSession, options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(name: "Incant smoke", sessionRole: session.role)
        configuration.delegateClass = SceneDelegate.self
        return configuration
    }
}
final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options: UIScene.ConnectionOptions) {
        guard let scene = scene as? UIWindowScene else { return }
        // A process/FFI probe with no editor UI.
        window = UIWindow(windowScene: scene)
        window?.rootViewController = UIViewController()
        window?.makeKeyAndVisible()
        let status = incantSmokeRun()
        let report = "{\"ok\":\(status == 0),\"status\":\(status)}"
        print(status == 0 ? "INCANT_SMOKE_PASS" : "INCANT_SMOKE_FAIL")
        let path = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("smoke-result.json")
        try? report.write(to: path, atomically: true, encoding: .utf8)
    }
}
