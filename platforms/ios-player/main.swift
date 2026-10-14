import UIKit

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, configurationForConnecting session: UISceneSession,
                     options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(name: "Incant game", sessionRole: session.role)
        configuration.delegateClass = SceneDelegate.self
        return configuration
    }
}
final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    private var game: GameView?
    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options: UIScene.ConnectionOptions) {
        guard let scene = scene as? UIWindowScene else { return }
        let window = UIWindow(windowScene: scene)
        let controller = UIViewController()
        let game = GameView(frame: window.bounds)
        game.onFailure = { message in NSLog("INCANT_NATIVE_HOST_FAILED: %@", message) }
        controller.view = game
        window.rootViewController = controller
        self.window = window
        self.game = game
        window.makeKeyAndVisible()
    }
    func sceneDidBecomeActive(_ scene: UIScene) { game?.setActive(true) }
    func sceneWillResignActive(_ scene: UIScene) { game?.setActive(false) }
    func sceneDidDisconnect(_ scene: UIScene) { game?.close(); game = nil; window = nil }
}
