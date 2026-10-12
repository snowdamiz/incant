use incant_game_runtime::{GameError, OfflineGameSession};
use incant_runtime::*;
use incant_script::CompiledScript;
use std::sync::Arc;
#[test]
fn default_game_refuses_text_before_vm_executes() {
    let id = |n| StableId([n; 16]);
    let scene = CookedScene::new(
        id(0),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap()
    .with_text(
        vec![FontAssetRef {
            id: id(9),
            fingerprint: [1; 32],
        }],
        vec![CookedText {
            entity: id(1),
            text: Arc::new(NativeText {
                content: Arc::from("text"),
                fonts: Arc::from([id(9)]),
                layout: TextLayout {
                    font_size: 16.,
                    line_height: 20.,
                    width: 100.,
                    wrap: TextWrap::None,
                },
            }),
        }],
    )
    .unwrap();
    let script = CompiledScript::compile("throw Error('VM must not start')").unwrap();
    assert!(matches!(
        OfflineGameSession::new(scene, script, Default::default(), vec![], 48000),
        Err(GameError::TextHost)
    ));
}
