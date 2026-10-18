//! Explicit opt-in selection among trusted build-linked descriptors. This never
//! loads guest source, Wasm files, executable caches or native pointers from data.
use incant_script::SessionProgram;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub fn split_args(args: &[String]) -> Result<(&[String], Option<&str>)> {
    if args.first().is_some_and(|arg| arg == "--static-module") {
        if args.len() < 3 {
            return Err("--static-module requires a linked name and player arguments".into());
        }
        Ok((&args[2..], Some(&args[1])))
    } else {
        Ok((args, None))
    }
}
pub fn load(module: Option<&str>) -> Result<SessionProgram> {
    let authored = option_env!("INCANT_AUTHORED_WASM").filter(|name| !name.is_empty());
    if authored.is_some() && module.is_some() {
        return Err("authored package program cannot be overridden".into());
    }
    let module = module.or(authored);
    match module {
        None => Ok(crate::embedded::load()?.into()),
        Some(name) => {
            #[cfg(feature = "static-module")]
            {
                let program: SessionProgram = incant_wasm_runtime::embedded::linked(name)
                    .ok_or("unknown trusted linked module")?
                    .into();
                if authored.is_some() {
                    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/wasm-instances.bin"));
                    if let Some(catalog) = built_instances(bytes)? {
                        return Ok(program.with_instances(catalog)?);
                    }
                }
                Ok(program)
            }
            #[cfg(not(feature = "static-module"))]
            {
                let _ = name;
                Err("linked module selection requires the static-module feature".into())
            }
        }
    }
}
#[cfg(feature = "static-module")]
fn built_instances(bytes: &[u8]) -> Result<Option<incant_wasm_runtime::instances::Catalog>> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let (scene, frame) = bytes
        .split_at_checked(32)
        .ok_or("invalid built instance catalog")?;
    Ok(Some(incant_wasm_runtime::instances::Catalog::from_frame(
        scene.try_into()?,
        frame,
    )?))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_prefix_is_bounded_and_default_arguments_are_unchanged() {
        let args = vec!["scene".into(), "cache".into()];
        assert_eq!(split_args(&args).unwrap(), (&args[..], None));
        let args = vec![
            "--static-module".into(),
            "flipbook-game".into(),
            "scene".into(),
        ];
        assert_eq!(
            split_args(&args).unwrap(),
            (&args[2..], Some("flipbook-game"))
        );
        assert!(split_args(&args[..2]).is_err());
        assert!(load(Some("untrusted.wasm")).is_err());
    }
    #[test]
    #[cfg(feature = "static-module")]
    fn built_catalog_empty_default_is_absent_but_nonempty_truncation_refuses() {
        // The non-authored build emits an include_bytes array of this exact size.
        const EMPTY: &[u8; 0] = b"";
        assert_eq!(built_instances(EMPTY).unwrap(), None);
        for length in 1..=32 {
            assert!(
                built_instances(&vec![0; length]).is_err(),
                "length {length}"
            );
        }
    }
    #[test]
    #[cfg(feature = "static-module")]
    fn built_catalog_preserves_scene_rows_and_rejects_malformed_payload() {
        use incant_runtime::StableId;
        use incant_types::CoroutineValue;
        use incant_wasm_runtime::instances::Catalog;
        use std::collections::BTreeMap;
        let catalog = Catalog::new(
            [7; 32],
            BTreeMap::from([(
                StableId(1u128.to_be_bytes()),
                BTreeMap::from([("value".into(), CoroutineValue::Bool(true))]),
            )]),
        )
        .unwrap();
        let mut bytes = catalog.scene().to_vec();
        bytes.extend(catalog.frame());
        assert_eq!(built_instances(&bytes).unwrap(), Some(catalog));
        bytes.push(0);
        assert!(built_instances(&bytes).is_err());
        bytes.pop();
        bytes[32] ^= 1;
        assert!(built_instances(&bytes).is_err());
    }
    #[test]
    #[cfg(feature = "static-module")]
    fn legacy_static_guest_accepts_trusted_haptic_authority_without_issuing_output() {
        let scene = incant_runtime::CookedScene::new(
            incant_runtime::StableId(1u128.to_be_bytes()),
            60,
            vec![],
        )
        .unwrap();
        let mut session = incant_script::NativeSession::new(
            scene,
            load(Some(
                option_env!("INCANT_HAPTICS_STATIC_FIXTURE").unwrap_or("bulk"),
            ))
            .unwrap(),
            Default::default(),
        )
        .unwrap();
        assert!(session.supports_haptics().unwrap());
        let caps = incant_input::haptics::HapticCapabilities::default()
            .with_gamepad(0, 1)
            .unwrap();
        session.set_haptic_capabilities(caps).unwrap();
        session.set_haptic_capabilities(Default::default()).unwrap();
        session.tick(&[]).unwrap();
        assert!(session.take_haptics().unwrap().is_empty());
    }
}
