//! Fixed authoring compiler invocation; no ambient Node or cache configuration.
use std::{path::Path, process::Command};

pub(super) struct CompilerPaths<'a> {
    pub node: &'a Path,
    pub engine: &'a Path,
    pub project: &'a Path,
    pub output: &'a Path,
    pub temporary: &'a Path,
    pub phases: &'a Path,
    pub cache: &'a Path,
}
impl CompilerPaths<'_> {
    pub fn configure(&self, command: &mut Command) {
        command
            .args(["--max-old-space-size=512", "--disable-proto=throw"])
            // Only the trusted entry uses relative spelling (nodejs/node#62446).
            .arg(super::DRIVER)
            .arg("--project")
            .arg(self.project)
            .arg(self.output)
            .current_dir(self.engine)
            .env_clear()
            .env(
                "PATH",
                self.node.parent().expect("canonical executable parent"),
            )
            .env("TMPDIR", self.temporary)
            .env("TEMP", self.temporary)
            .env("TMP", self.temporary)
            .env("INCANT_NATIVE_COOK_PHASE_FILE", self.phases)
            .env("SWC_NATIVE_BINDING_CACHE", self.cache);
        // No LOCALAPPDATA/HOME: a rejected custom SWC root must not escape to
        // the carrier's default Windows cache. Only OS installation paths pass.
        for key in ["SystemRoot", "SystemDrive", "WINDIR"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiler_environment_discards_ambient_node_and_cache_overrides() {
        let root = tempfile::tempdir().unwrap();
        let node = root.path().join("node");
        let cache = root.path().join("trusted-cache");
        let mut command = Command::new(&node);
        for key in [
            "NODE_OPTIONS",
            "NODE_PATH",
            "HOME",
            "LOCALAPPDATA",
            "SWC_BINARY_PATH",
            "SWC_NATIVE_BINDING_CACHE",
            "SECRET_TOKEN",
        ] {
            command.env(key, "untrusted inherited value");
        }
        CompilerPaths {
            node: &node,
            engine: root.path(),
            project: root.path(),
            output: root.path(),
            temporary: root.path(),
            phases: root.path(),
            cache: &cache,
        }
        .configure(&mut command);
        let environment: std::collections::BTreeMap<_, _> = command.get_envs().collect();
        for key in [
            "NODE_OPTIONS",
            "NODE_PATH",
            "HOME",
            "LOCALAPPDATA",
            "SWC_BINARY_PATH",
            "SECRET_TOKEN",
        ] {
            assert!(
                !environment.contains_key(std::ffi::OsStr::new(key)),
                "retained {key}"
            );
        }
        assert_eq!(
            environment[std::ffi::OsStr::new("SWC_NATIVE_BINDING_CACHE")],
            Some(cache.as_os_str())
        );
        assert_eq!(
            environment[std::ffi::OsStr::new("PATH")],
            Some(root.path().as_os_str())
        );
        assert_eq!(super::super::DEADLINE, std::time::Duration::from_secs(8));
    }
}
