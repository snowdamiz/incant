//! Embedded engine-owned artifacts only. This is not an external shader loader.
use super::Stages;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Vertex,
    Fragment,
}
#[derive(Clone, Copy)]
pub(crate) struct Artifact {
    pub backend: wgpu::Backend,
    pub stage: Stage,
    pub entry: &'static str,
    pub bytes: &'static [u8],
    pub hash: [u8; 32],
}
pub(crate) struct Bundle {
    pub contract: &'static str,
    pub expected: &'static str,
    pub source: [u8; 32],
    pub producer: [u8; 32],
    pub artifacts: &'static [Artifact],
}
pub(crate) fn features(
    backend: wgpu::Backend,
    available: wgpu::Features,
) -> std::result::Result<wgpu::Features, &'static str> {
    if !matches!(
        backend,
        wgpu::Backend::Metal | wgpu::Backend::Vulkan | wgpu::Backend::Dx12
    ) {
        return Err("unsupported native backend");
    }
    if !available.contains(wgpu::Features::PASSTHROUGH_SHADERS) {
        return Err("passthrough shaders unavailable");
    }
    Ok(wgpu::Features::PASSTHROUGH_SHADERS)
}
impl Bundle {
    pub(crate) fn select(
        &self,
        backend: wgpu::Backend,
        stage: Stage,
    ) -> std::result::Result<&'static Artifact, &'static str> {
        if self.contract != self.expected
            || self.source == [0; 32]
            || self.producer == [0; 32]
            || self.artifacts.len() != 6
        {
            return Err("manifest contract mismatch");
        }
        let mut matches = self
            .artifacts
            .iter()
            .filter(|a| a.backend == backend && a.stage == stage);
        let a = matches.next().ok_or("missing stage")?;
        if matches.next().is_some() {
            return Err("duplicate stage");
        }
        Ok(a)
    }
    pub(crate) fn stages(
        &self,
        device: &wgpu::Device,
        backend: wgpu::Backend,
    ) -> std::result::Result<Stages, &'static str> {
        features(backend, device.features())?;
        let vertex = self.select(backend, Stage::Vertex)?;
        let fragment = self.select(backend, Stage::Fragment)?;
        // Both descriptors are checked before either driver call.
        let vertex_desc = descriptor(vertex)?;
        let fragment_desc = descriptor(fragment)?;
        // SAFETY: callers are private fixed-builtin selectors. Their pinned host
        // cook validates exact IR/host layouts and emits bounded accesses/loops.
        // Output has no side metadata; geometry/sprites/UI/text use exact hal29 bounded
        // vertex pulling, supplied lengths and full sampler-index binding map.
        // Embedded bytes/entry names are never supplied by project artifacts.
        let (vertex_module, fragment_module) = unsafe {
            (
                device.create_shader_module_passthrough(vertex_desc),
                device.create_shader_module_passthrough(fragment_desc),
            )
        };
        Ok(Stages {
            vertex: vertex_module,
            fragment: fragment_module,
            vertex_entry: vertex.entry,
            fragment_entry: fragment.entry,
        })
    }
}
pub(crate) fn descriptor(
    a: &Artifact,
) -> std::result::Result<wgpu::ShaderModuleDescriptorPassthrough<'static>, &'static str> {
    if a.bytes.is_empty()
        || a.bytes.len() > 1024 * 1024
        || <[u8; 32]>::from(Sha256::digest(a.bytes)) != a.hash
        || a.entry.is_empty()
        || !a
            .entry
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("invalid artifact");
    }
    let mut desc = wgpu::ShaderModuleDescriptorPassthrough {
        label: Some("Cooked builtin shader"),
        ..Default::default()
    };
    match a.backend {
        wgpu::Backend::Metal | wgpu::Backend::Dx12 => {
            let text = std::str::from_utf8(a.bytes).map_err(|_| "invalid shader text")?;
            if a.backend == wgpu::Backend::Metal {
                desc.msl = Some(Cow::Borrowed(text));
            } else {
                desc.hlsl = Some(Cow::Borrowed(text));
            }
        }
        wgpu::Backend::Vulkan => {
            if a.bytes.len() < 20 || !a.bytes.len().is_multiple_of(4) {
                return Err("invalid SPIR-V length");
            }
            let words: Vec<u32> = a
                .bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
                .collect();
            if words[0] != 0x07230203 || words[1] != 0x00010000 || words[4] != 0 {
                return Err("invalid SPIR-V header");
            }
            desc.spirv = Some(Cow::Owned(words));
        }
        _ => return Err("unsupported artifact backend"),
    }
    Ok(desc)
}
