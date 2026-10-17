//! Cold trusted-build admission only. Never linked into the gameplay runtime.
#![allow(
    dead_code,
    reason = "Shared build support: script and C producers use different entry points."
)]
use crate::{json, serde_support};
use serde_support::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fmt, fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
};

pub const SELECTOR: &str = "INCANT_QUICKJS_PGO_DESCRIPTOR";
pub const TARGET: &str = "aarch64-apple-darwin";
pub const MAX_DESCRIPTOR: u64 = 64 * 1024;
pub const MAX_PROFILE: u64 = 64 * 1024 * 1024;
pub const SOURCES: [&str; 18] = [
    "builtin-array-fromasync.h",
    "builtin-iterator-zip-keyed.h",
    "builtin-iterator-zip.h",
    "cutils.h",
    "dtoa.h",
    "libregexp-opcode.h",
    "libregexp.h",
    "libunicode-table.h",
    "libunicode.h",
    "list.h",
    "quickjs-atom.h",
    "quickjs-c-atomics.h",
    "quickjs-opcode.h",
    "quickjs.h",
    "libregexp.c",
    "libunicode.c",
    "quickjs.c",
    "dtoa.c",
];
pub const UNITS: [&str; 4] = ["libregexp.c", "libunicode.c", "quickjs.c", "dtoa.c"];
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn ordered(value: json::Value) -> json::Value {
    match value {
        json::Value::Object(map) => json::Value::Object(
            map.into_iter()
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .map(|(k, v)| (k, ordered(v)))
                .collect(),
        ),
        json::Value::Array(values) => json::Value::Array(values.into_iter().map(ordered).collect()),
        v => v,
    }
}

#[derive(Debug)]
pub enum Error {
    Invalid(&'static str),
    Io(io::Error),
    Json(json::Error),
    RecipeMismatch {
        role: Role,
        expected: Vec<String>,
        actual: Vec<String>,
    },
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(s) => write!(f, "QuickJS PGO admission: {s}"),
            Self::Io(e) => write!(f, "QuickJS PGO input: {e}"),
            Self::Json(e) => write!(f, "QuickJS PGO descriptor: {e}"),
            Self::RecipeMismatch {
                role,
                expected,
                actual,
            } => write!(
                f,
                "QuickJS PGO ordered C recipe mismatch for {role:?}: expected={expected:?}, actual={actual:?}"
            ),
        }
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<json::Error> for Error {
    fn from(e: json::Error) -> Self {
        Self::Json(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(reason))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", rename_all = "kebab-case")]
pub enum Mode {
    Generate,
    Use,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", deny_unknown_fields)]
pub struct DigestFile {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", deny_unknown_fields)]
pub struct Content {
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", deny_unknown_fields)]
pub struct Recipe {
    pub optimization: u8,
    pub debug: bool,
    /// Exact ordinary role cc::Tool args, before any runtime-only additions.
    pub base_args: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", rename_all = "kebab-case")]
pub enum Role {
    Cooker,
    Runtime,
}
impl Role {
    pub fn from_optimization(value: &str) -> Result<Self> {
        match value {
            "0" => Ok(Self::Cooker),
            "3" => Ok(Self::Runtime),
            _ => Err(Error::Invalid("unexpected Cargo role optimization")),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", rename_all = "kebab-case")]
pub enum ProfileScope {
    RuntimeOnly,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", deny_unknown_fields)]
pub struct Recipes {
    pub cooker: Recipe,
    pub runtime: Recipe,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(crate = "crate::serde_support", deny_unknown_fields)]
pub struct Descriptor {
    pub version: u8,
    pub mode: Mode,
    pub target: String,
    pub compiler: DigestFile,
    pub archiver: DigestFile,
    pub profdata_tool: DigestFile,
    pub profile_runtime: DigestFile,
    pub sdk: PathBuf,
    pub resource_directory: PathBuf,
    /// Hash-bound SDK/resource metadata and supporting compiler files.
    pub tool_inputs: Vec<DigestFile>,
    #[serde(deserialize_with = "source_map")]
    pub sources: BTreeMap<String, Content>,
    pub recipes: Recipes,
    pub profile_scope: ProfileScope,
    pub profile: Option<DigestFile>,
    pub raw_directory: Option<PathBuf>,
}
fn source_map<'de, D: serde_support::Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<String, Content>, D::Error> {
    struct Unique;
    impl<'de> serde_support::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, Content>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unique source names")
        }
        fn visit_map<M: serde_support::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut result = BTreeMap::new();
            while let Some((name, value)) = map.next_entry()? {
                if result.insert(name, value).is_some() {
                    return Err(serde_support::de::Error::custom("duplicate source name"));
                }
            }
            Ok(result)
        }
    }
    d.deserialize_map(Unique)
}

fn canonical(path: &Path, directory: bool) -> Result<()> {
    require(
        path.is_absolute()
            && path.to_str().is_some_and(|s| {
                !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
            }),
        "absolute bounded UTF-8 path required",
    )?;
    require(
        path.components()
            .all(|c| !matches!(c, Component::CurDir | Component::ParentDir)),
        "noncanonical path",
    )?;
    for ancestor in path.ancestors() {
        let info = fs::symlink_metadata(ancestor)?;
        require(!info.file_type().is_symlink(), "linked input or ancestor")?;
    }
    require(path.canonicalize()? == path, "input path is not canonical")?;
    let info = fs::metadata(path)?;
    require(
        if directory {
            info.is_dir()
        } else {
            info.is_file()
        },
        "input kind",
    )
}
fn digest_valid(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn bounded(path: &Path, cap: u64) -> Result<Vec<u8>> {
    canonical(path, false)?;
    let mut value = Vec::new();
    fs::File::open(path)?
        .take(cap + 1)
        .read_to_end(&mut value)?;
    require(value.len() as u64 <= cap, "input byte cap")?;
    Ok(value)
}
fn content(path: &Path, cap: u64) -> Result<Content> {
    canonical(path, false)?;
    let mut source = fs::File::open(path)?.take(cap + 1);
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = source.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        require(bytes <= cap, "hashed input byte cap")?;
        hash.update(&buffer[..n]);
    }
    Ok(Content {
        bytes,
        sha256: format!("{:x}", hash.finalize()),
    })
}
fn verify_file(file: &DigestFile, cap: u64) -> Result<()> {
    require(
        file.bytes > 0 && file.bytes <= cap && digest_valid(&file.sha256),
        "file identity bounds",
    )?;
    let actual = content(&file.path, file.bytes)?;
    require(
        actual.bytes == file.bytes && actual.sha256 == file.sha256,
        "file bytes changed",
    )
}
impl Descriptor {
    pub fn parse(bytes: &[u8], mode: Mode) -> Result<Self> {
        require(bytes.len() as u64 <= MAX_DESCRIPTOR, "descriptor byte cap")?;
        let v: Self = json::from_slice(bytes)?;
        let fields: json::Value = json::from_slice(bytes)?;
        require(
            fields.as_object().is_some_and(|m| m.len() == 15),
            "all descriptor fields required",
        )?;
        require(
            v.version == 2 && v.mode == mode && v.target == TARGET,
            "mode/version/target",
        )?;
        for (role, optimization) in [(Role::Cooker, 0), (Role::Runtime, 3)] {
            let recipe = v.recipe(role);
            require(
                recipe.optimization == optimization && recipe.debug,
                "ordinary role-specific C recipe required",
            )?;
            require(
                !recipe.base_args.is_empty()
                    && recipe.base_args.len() <= 128
                    && recipe.base_args.iter().all(|a| {
                        !a.is_empty()
                            && a.len() <= 4096
                            && !a.chars().any(char::is_control)
                            && a != "-w"
                            && !a.contains("profile-")
                            && !a.starts_with('@')
                    })
                    && recipe.base_args.first() == Some(&format!("-O{optimization}"))
                    && recipe
                        .base_args
                        .last()
                        .is_some_and(|a| a == "-D_GNU_SOURCE"),
                "base role argument vector",
            )?;
        }
        require(
            v.sources.len() == SOURCES.len() && SOURCES.iter().all(|s| v.sources.contains_key(*s)),
            "exact QuickJS source closure",
        )?;
        require(
            v.sources
                .values()
                .all(|s| s.bytes > 0 && s.bytes <= 4 * 1024 * 1024 && digest_valid(&s.sha256)),
            "source digest bounds",
        )?;
        require(
            !v.tool_inputs.is_empty() && v.tool_inputs.len() <= 512,
            "tool closure count",
        )?;
        let unique: BTreeSet<_> = v.tool_inputs.iter().map(|f| &f.path).collect();
        require(unique.len() == v.tool_inputs.len(), "duplicate tool input")?;
        require(
            match mode {
                Mode::Generate => v.profile.is_none() && v.raw_directory.is_some(),
                Mode::Use => v.profile.is_some() && v.raw_directory.is_none(),
            },
            "profile/mode fields",
        )?;
        Ok(v)
    }
    pub fn verify(&self, vendor: &Path) -> Result<()> {
        for f in [
            &self.compiler,
            &self.archiver,
            &self.profdata_tool,
            &self.profile_runtime,
        ] {
            verify_file(f, 256 * 1024 * 1024)?;
        }
        require(
            self.profile_runtime
                .path
                .file_name()
                .is_some_and(|n| n == "libclang_rt.profile_osx.a"),
            "profile runtime archive",
        )?;
        canonical(&self.sdk, true)?;
        canonical(&self.resource_directory, true)?;
        for f in &self.tool_inputs {
            verify_file(f, 256 * 1024 * 1024)?;
        }
        require(
            self.tool_inputs
                .iter()
                .any(|f| f.path.starts_with(&self.sdk))
                && self
                    .tool_inputs
                    .iter()
                    .any(|f| f.path.starts_with(&self.resource_directory)),
            "SDK/resource inputs missing",
        )?;
        for (name, expected) in &self.sources {
            require(
                content(&vendor.join("quickjs").join(name), 4 * 1024 * 1024)? == *expected,
                "QuickJS source changed",
            )?;
        }
        if let Some(f) = &self.profile {
            verify_file(f, MAX_PROFILE)?;
        }
        if let Some(p) = &self.raw_directory {
            canonical(p, true)?;
        }
        Ok(())
    }
    /// Physical profile location is not compatibility: the verified bytes are.
    /// Actual cc vectors and resolved paths remain in the separate role receipt.
    pub fn identity(&self) -> Result<[u8; 32]> {
        let file_id = |f: &DigestFile| json::json!({"bytes":f.bytes,"sha256":f.sha256});
        let value = json::json!({"format":"incant-quickjs-c-pgo-v2", "mode":self.mode,
            "target":self.target,"sources":self.sources,"recipes":self.recipes,"profile_scope":self.profile_scope,
            "compiler":file_id(&self.compiler),"archiver":file_id(&self.archiver),
            "profdata_tool":file_id(&self.profdata_tool),"profile_runtime":file_id(&self.profile_runtime),
            "sdk":self.sdk,"resource_directory":self.resource_directory,
            "tool_inputs":self.tool_inputs.iter().map(|f| json::json!({"path":f.path,"content":file_id(f)})).collect::<Vec<_>>(),
            "profile":self.profile.as_ref().map(file_id)});
        Ok(Sha256::digest(json::to_vec(&ordered(value))?).into())
    }
    pub fn recipe(&self, role: Role) -> &Recipe {
        match role {
            Role::Cooker => &self.recipes.cooker,
            Role::Runtime => &self.recipes.runtime,
        }
    }
    /// Host cooking remains ordinary; a runtime profile is never fed to O0 host C.
    pub fn profile_for(&self, role: Role) -> Option<&DigestFile> {
        if role == Role::Runtime {
            self.profile.as_ref()
        } else {
            None
        }
    }
    pub fn additions(&self, role: Role) -> Vec<String> {
        if role == Role::Cooker {
            return Vec::new();
        }
        let mut flags = vec!["-Werror=unknown-warning-option".into()];
        match self.mode {
            Mode::Generate => flags.push(format!(
                "-fprofile-generate={}",
                self.raw_directory.as_ref().unwrap().display()
            )),
            Mode::Use => {
                flags.push(format!(
                    "-fprofile-use={}",
                    self.profile.as_ref().unwrap().path.display()
                ));
                flags.extend(
                    [
                        "-Werror=profile-instr-missing",
                        "-Werror=profile-instr-out-of-date",
                        "-Werror=profile-instr-unprofiled",
                        "-Werror=backend-plugin",
                        "-mllvm",
                        "-pgo-warn-missing-function",
                    ]
                    .into_iter()
                    .map(str::to_owned),
                );
            }
        }
        flags
    }
    pub fn check_compiler(&self, invocation: &Path, search_path: &std::ffi::OsStr) -> Result<()> {
        let selected = resolve_compiler(invocation, search_path)?;
        require(
            selected == self.compiler.path,
            "natural compiler differs from admitted executable",
        )?;
        verify_file(&self.compiler, 256 * 1024 * 1024)
    }
    pub fn check_args(&self, role: Role, args: &[String], instrumented: bool) -> Result<()> {
        require(
            args.len() <= 128
                && args
                    .iter()
                    .all(|a| a.len() <= 4096 && !a.chars().any(char::is_control)),
            "actual C vector bounds",
        )?;
        let mut expected = self.recipe(role).base_args.clone();
        if instrumented {
            // cc emits builder flags, then flag_if_supported flags, then defines.
            // Preserve the exact ordinary vector; only insert at that known slot.
            let end = if expected.len() > 1
                && expected[expected.len() - 2] == "-Wno-implicit-const-int-float-conversion"
            {
                expected.len() - 2
            } else {
                expected.len() - 1
            };
            expected.splice(end..end, self.additions(role));
        }
        if args == expected {
            Ok(())
        } else {
            Err(Error::RecipeMismatch {
                role,
                expected,
                actual: args.to_vec(),
            })
        }
    }
}

/// Follow exec's first executable PATH candidate, then verify its canonical file.
/// Do not feed that resolved spelling back into cc, which changes Apple SDK args.
#[cfg(unix)]
pub fn resolve_compiler(invocation: &Path, search_path: &std::ffi::OsStr) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    require(
        invocation
            .to_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)),
        "compiler invocation bounds",
    )?;
    if invocation.is_absolute() {
        let info = fs::metadata(invocation)?;
        require(
            info.is_file() && info.permissions().mode() & 0o111 != 0,
            "compiler invocation is not executable",
        )?;
        return Ok(invocation.canonicalize()?);
    }
    require(
        invocation.components().count() == 1
            && matches!(invocation.components().next(), Some(Component::Normal(_))),
        "natural compiler must be a bare name or absolute path",
    )?;
    let paths: Vec<_> = env::split_paths(search_path).collect();
    require(
        !paths.is_empty()
            && paths.len() <= 128
            && paths.iter().all(|p| {
                p.is_absolute()
                    && p.to_str()
                        .is_some_and(|s| s.len() <= 4096 && !s.chars().any(char::is_control))
            }),
        "bounded absolute compiler PATH",
    )?;
    for directory in paths {
        let candidate = directory.join(invocation);
        match fs::metadata(&candidate) {
            Ok(info) if info.is_file() && info.permissions().mode() & 0o111 != 0 => {
                return Ok(candidate.canonicalize()?);
            }
            Ok(_) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Err(Error::Invalid("natural compiler not found"))
}

#[cfg(not(unix))]
pub fn resolve_compiler(_: &Path, _: &std::ffi::OsStr) -> Result<PathBuf> {
    Err(Error::Invalid(
        "natural compiler diagnostic requires native Mac",
    ))
}

pub struct Selection {
    pub descriptor: Descriptor,
    pub role: Role,
    path: PathBuf,
}
/// FreshSession enables this exact cc diagnostic sink for every mode. It emits
/// command evidence without changing the C recipe; no other CC override follows
/// from admitting it. Ambient callers are still refused by the outer probe.
pub fn compiler_environment(key: &str, value: &std::ffi::OsStr) -> Result<()> {
    if key == "CC_ENABLE_DEBUG_OUTPUT" {
        return require(
            value.is_empty() || value == std::ffi::OsStr::new("1"),
            "exact cc diagnostic output setting",
        );
    }
    let forbidden = key.starts_with("CFLAGS")
        || key.ends_with("_CFLAGS")
        || key.starts_with("CPPFLAGS")
        || key.ends_with("_CPPFLAGS")
        || matches!(
            key,
            "CC" | "AR"
                | "HOST_CC"
                | "TARGET_CC"
                | "HOST_AR"
                | "TARGET_AR"
                | "RUSTFLAGS"
                | "CARGO_ENCODED_RUSTFLAGS"
                | "CARGO_CFG_SANITIZE"
        )
        || key.starts_with("LLVM_PROFILE_")
        || key.starts_with("CARGO_PROFILE_")
        || key.starts_with("CC_")
        || key.starts_with("AR_")
        || key.ends_with("_CC")
        || key.ends_with("_AR");
    require(
        !forbidden || value.is_empty(),
        "ambient compiler/profile override",
    )
}
impl Selection {
    pub fn load(vendor: &Path) -> Result<Self> {
        let generate = env::var_os("CARGO_FEATURE_DIAGNOSTIC_QUICKJS_GENERATE").is_some();
        let use_mode = env::var_os("CARGO_FEATURE_DIAGNOSTIC_QUICKJS_USE").is_some();
        require(
            generate != use_mode,
            "select exactly one diagnostic feature",
        )?;
        require(
            env::var("HOST").ok().as_deref() == Some(TARGET)
                && env::var("TARGET").ok().as_deref() == Some(TARGET),
            "native host/target only",
        )?;
        for (key, value) in env::vars_os() {
            let key = key.to_string_lossy();
            compiler_environment(&key, &value)?;
            if key.starts_with("CARGO_FEATURE_") {
                require(
                    !key.starts_with("CARGO_FEATURE_DUMP_")
                        && !matches!(
                            key.as_ref(),
                            "CARGO_FEATURE_DISABLE_ASSERTIONS"
                                | "CARGO_FEATURE_BINDGEN"
                                | "CARGO_FEATURE_UPDATE_BINDINGS"
                                | "CARGO_FEATURE_STATIC_MODULE"
                                | "CARGO_FEATURE_WASMTIME_HOST"
                                | "CARGO_FEATURE_PROBE_ANDROID_BYTECODE"
                        ),
                    "unsupported feature mixture",
                )?;
            }
        }
        println!("cargo:rerun-if-env-changed={SELECTOR}");
        let path =
            PathBuf::from(env::var_os(SELECTOR).ok_or(Error::Invalid("missing descriptor"))?);
        let descriptor = Descriptor::parse(
            &bounded(&path, MAX_DESCRIPTOR)?,
            if generate { Mode::Generate } else { Mode::Use },
        )?;
        descriptor.verify(vendor)?;
        let search_path = env::var_os("PATH").ok_or(Error::Invalid("missing compiler PATH"))?;
        descriptor.check_compiler(Path::new("cc"), &search_path)?;
        require(
            resolve_compiler(Path::new("ar"), &search_path)? == descriptor.archiver.path,
            "natural archiver differs from admitted executable",
        )?;
        let role = Role::from_optimization(
            &env::var("OPT_LEVEL").map_err(|_| Error::Invalid("missing Cargo optimization"))?,
        )?;
        require(
            env::var_os("SDKROOT").as_deref() == Some(descriptor.sdk.as_os_str()),
            "SDKROOT differs from admitted SDK",
        )?;
        println!("cargo:rerun-if-changed={}", path.display());
        for f in [
            &descriptor.compiler,
            &descriptor.archiver,
            &descriptor.profdata_tool,
            &descriptor.profile_runtime,
        ]
        .into_iter()
        .chain(descriptor.tool_inputs.iter())
        .chain(descriptor.profile.iter())
        {
            println!("cargo:rerun-if-changed={}", f.path.display());
        }
        Ok(Self {
            descriptor,
            role,
            path,
        })
    }
    pub fn recheck(&self, vendor: &Path) -> Result<()> {
        require(
            Descriptor::parse(&bounded(&self.path, MAX_DESCRIPTOR)?, self.descriptor.mode)?
                == self.descriptor,
            "descriptor changed during build",
        )?;
        self.descriptor.verify(vendor)?;
        let search_path = env::var_os("PATH").ok_or(Error::Invalid("missing compiler PATH"))?;
        self.descriptor
            .check_compiler(Path::new("cc"), &search_path)?;
        require(
            resolve_compiler(Path::new("ar"), &search_path)? == self.descriptor.archiver.path,
            "natural archiver changed during build",
        )?;
        require(
            Role::from_optimization(
                &env::var("OPT_LEVEL").map_err(|_| Error::Invalid("missing Cargo optimization"))?,
            )? == self.role,
            "Cargo role changed during build",
        )
    }
    /// `default_c_flags` below is the shared target bytecode-compatibility flag;
    /// `c_unit_profiled` separately describes the actual producer role's C flags.
    pub fn receipt(&self, out: &Path, package: &str, args: Option<&[String]>) -> Result<()> {
        let digest = self.descriptor.identity()?;
        let value = json::json!({"format":"incant-quickjs-pgo-build-v2","package":package,
            "mode":self.descriptor.mode,"identity":hex(&digest),
            "descriptor_sha256":format!("{:x}", Sha256::digest(bounded(&self.path, MAX_DESCRIPTOR)?)),
            "target":self.descriptor.target,"out_dir":out,"compiler":self.descriptor.compiler,
            "role":self.role,"profile_scope":self.descriptor.profile_scope,
            "recipe":self.descriptor.recipe(self.role),"actual_args":args,
            "compiler_invocation":args.map(|_| "cc"),
            "profile":self.descriptor.profile_for(self.role),
            "c_unit_profiled":if args.is_some() { Some(self.role == Role::Runtime) } else { None },
            "default_c_flags":false,"source_free_claim":false});
        let bytes = json::to_vec_pretty(&value)?;
        require(bytes.len() as u64 <= MAX_DESCRIPTOR, "receipt byte cap")?;
        let mut file = fs::File::create(out.join("quickjs-pgo.json"))?;
        file.write_all(&bytes)?;
        Ok(())
    }
}
