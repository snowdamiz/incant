//! Encode effective native fields explicitly. Never hash descriptor padding,
//! COM/payload addresses, diagnostic labels, or pipeline creation order.
use super::pipeline_desc::RenderPipelineStateStreamDesc;
use crate::pipeline_library_key::Encoder;
use core::{ffi::CStr, slice};
use windows::Win32::Graphics::Direct3D12::*;

unsafe fn bytes<'a>(pointer: *const core::ffi::c_void, length: usize) -> &'a [u8] {
    if length == 0 {
        &[]
    } else {
        // SAFETY: native descriptor caller keeps this exact byte range valid
        // through key generation and the corresponding D3D12 creation call.
        unsafe { slice::from_raw_parts(pointer.cast(), length) }
    }
}
unsafe fn records<'a, T>(pointer: *const T, length: u32) -> &'a [T] {
    if length == 0 {
        &[]
    } else {
        // SAFETY: same native descriptor lifetime/array validity contract.
        unsafe { slice::from_raw_parts(pointer, length as usize) }
    }
}
fn stencil(key: &mut Encoder, face: D3D12_DEPTH_STENCILOP_DESC) {
    key.u32(face.StencilFailOp.0 as u32);
    key.u32(face.StencilDepthFailOp.0 as u32);
    key.u32(face.StencilPassOp.0 as u32);
    key.u32(face.StencilFunc.0 as u32);
}
unsafe fn semantic(key: &mut Encoder, name: windows_core::PCSTR) {
    // Null stream-output semantic names denote gaps. Preserve that distinction
    // without dereferencing them, including versus a nonnull empty C string.
    key.boolean(!name.0.is_null());
    if !name.0.is_null() {
        key.bytes(unsafe { CStr::from_ptr(name.0.cast()) }.to_bytes());
    }
}

/// # Safety
/// All pointed-to bytecode, semantic strings and arrays must remain valid for
/// the same native call as `descriptor`; root is the retained serialized root
/// signature digest, not its COM address.
pub(super) unsafe fn render(
    descriptor: &RenderPipelineStateStreamDesc<'_>,
    root: &[u8; 32],
) -> [u8; 32] {
    let d = descriptor;
    let mut key = Encoder::new(b"render");
    key.boolean(d.root_signature.is_some());
    key.bytes(root);
    for shader in [
        d.vertex_shader,
        d.pixel_shader,
        d.task_shader,
        d.mesh_shader,
    ] {
        key.bytes(unsafe { bytes(shader.pShaderBytecode, shader.BytecodeLength) });
    }
    key.boolean(d.blend_state.AlphaToCoverageEnable.as_bool());
    key.boolean(d.blend_state.IndependentBlendEnable.as_bool());
    for target in d.blend_state.RenderTarget {
        key.boolean(target.BlendEnable.as_bool());
        key.boolean(target.LogicOpEnable.as_bool());
        for value in [
            target.SrcBlend.0,
            target.DestBlend.0,
            target.BlendOp.0,
            target.SrcBlendAlpha.0,
            target.DestBlendAlpha.0,
            target.BlendOpAlpha.0,
            target.LogicOp.0,
        ] {
            key.u32(value as u32);
        }
        key.u32(u32::from(target.RenderTargetWriteMask));
    }
    key.u32(d.sample_mask);
    let raster = d.rasterizer_state;
    key.u32(raster.FillMode.0 as u32);
    key.u32(raster.CullMode.0 as u32);
    key.boolean(raster.FrontCounterClockwise.as_bool());
    key.u32(raster.DepthBias as u32);
    key.float(raster.DepthBiasClamp);
    key.float(raster.SlopeScaledDepthBias);
    key.boolean(raster.DepthClipEnable.as_bool());
    key.boolean(raster.MultisampleEnable.as_bool());
    key.boolean(raster.AntialiasedLineEnable.as_bool());
    key.u32(raster.ForcedSampleCount);
    key.u32(raster.ConservativeRaster.0 as u32);
    let depth = d.depth_stencil_state;
    key.boolean(depth.DepthEnable.as_bool());
    key.u32(depth.DepthWriteMask.0 as u32);
    key.u32(depth.DepthFunc.0 as u32);
    key.boolean(depth.StencilEnable.as_bool());
    key.u32(u32::from(depth.StencilReadMask));
    key.u32(u32::from(depth.StencilWriteMask));
    stencil(&mut key, depth.FrontFace);
    stencil(&mut key, depth.BackFace);
    key.u32(d.primitive_topology_type.0 as u32);
    key.u32(d.rtv_formats.NumRenderTargets);
    for format in d.rtv_formats.RTFormats {
        key.u32(format.0 as u32);
    }
    key.u32(d.dsv_format.0 as u32);
    key.u32(d.sample_desc.Count);
    key.u32(d.sample_desc.Quality);
    key.u32(d.node_mask);
    key.bytes(unsafe { bytes(d.cached_pso.pCachedBlob, d.cached_pso.CachedBlobSizeInBytes) });
    key.u32(d.flags.0 as u32);
    key.boolean(d.view_instancing.is_some());
    if let Some(view) = d.view_instancing {
        key.u32(view.ViewInstanceCount);
        key.u32(view.Flags.0 as u32);
        for location in unsafe { records(view.pViewInstanceLocations, view.ViewInstanceCount) } {
            key.u32(location.ViewportArrayIndex);
            key.u32(location.RenderTargetArrayIndex);
        }
    }
    key.u32(d.input_layout.NumElements);
    for input in unsafe {
        records(
            d.input_layout.pInputElementDescs,
            d.input_layout.NumElements,
        )
    } {
        unsafe { semantic(&mut key, input.SemanticName) };
        key.u32(input.SemanticIndex);
        key.u32(input.Format.0 as u32);
        key.u32(input.InputSlot);
        key.u32(input.AlignedByteOffset);
        key.u32(input.InputSlotClass.0 as u32);
        key.u32(input.InstanceDataStepRate);
    }
    key.u32(d.index_buffer_strip_cut_value.0 as u32);
    let output = d.stream_output;
    key.u32(output.NumEntries);
    for declaration in unsafe { records(output.pSODeclaration, output.NumEntries) } {
        key.u32(declaration.Stream);
        unsafe { semantic(&mut key, declaration.SemanticName) };
        key.u32(declaration.SemanticIndex);
        key.u32(u32::from(declaration.StartComponent));
        key.u32(u32::from(declaration.ComponentCount));
        key.u32(u32::from(declaration.OutputSlot));
    }
    key.u32(output.NumStrides);
    for stride in unsafe { records(output.pBufferStrides, output.NumStrides) } {
        key.u32(*stride);
    }
    key.u32(output.RasterizedStream);
    key.finish()
}

/// # Safety
/// `descriptor` has the same valid pointers/lifetimes as the native creation
/// call. Root is the serialized signature digest retained by its layout.
pub(super) unsafe fn compute(
    descriptor: &D3D12_COMPUTE_PIPELINE_STATE_DESC,
    root: &[u8; 32],
) -> [u8; 32] {
    let mut key = Encoder::new(b"compute");
    key.boolean(descriptor.pRootSignature.is_some());
    key.bytes(root);
    key.bytes(unsafe { bytes(descriptor.CS.pShaderBytecode, descriptor.CS.BytecodeLength) });
    key.u32(descriptor.NodeMask);
    key.bytes(unsafe {
        bytes(
            descriptor.CachedPSO.pCachedBlob,
            descriptor.CachedPSO.CachedBlobSizeInBytes,
        )
    });
    key.u32(descriptor.Flags.0 as u32);
    key.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Graphics::Dxgi::Common::*;

    fn descriptor() -> RenderPipelineStateStreamDesc<'static> {
        RenderPipelineStateStreamDesc {
            root_signature: None,
            pixel_shader: Default::default(),
            blend_state: Default::default(),
            sample_mask: !0,
            rasterizer_state: Default::default(),
            depth_stencil_state: Default::default(),
            primitive_topology_type: Default::default(),
            rtv_formats: Default::default(),
            dsv_format: Default::default(),
            sample_desc: Default::default(),
            node_mask: 0,
            cached_pso: Default::default(),
            flags: Default::default(),
            view_instancing: None,
            vertex_shader: Default::default(),
            input_layout: Default::default(),
            index_buffer_strip_cut_value: Default::default(),
            stream_output: Default::default(),
            task_shader: Default::default(),
            mesh_shader: Default::default(),
        }
    }
    #[test]
    fn native_render_fields_and_root_identity_change_semantic_name_without_struct_bytes() {
        let original = unsafe { render(&descriptor(), &[1; 32]) };
        let changes: &[fn(&mut RenderPipelineStateStreamDesc<'_>)] = &[
            |d| d.sample_mask = 7,
            |d| d.blend_state.AlphaToCoverageEnable = true.into(),
            |d| d.blend_state.IndependentBlendEnable = true.into(),
            |d| d.blend_state.RenderTarget[7].BlendEnable = true.into(),
            |d| d.blend_state.RenderTarget[7].LogicOpEnable = true.into(),
            |d| d.blend_state.RenderTarget[7].SrcBlend = D3D12_BLEND_ONE,
            |d| d.blend_state.RenderTarget[7].DestBlend = D3D12_BLEND_ONE,
            |d| d.blend_state.RenderTarget[7].BlendOp = D3D12_BLEND_OP_ADD,
            |d| d.blend_state.RenderTarget[7].SrcBlendAlpha = D3D12_BLEND_ONE,
            |d| d.blend_state.RenderTarget[7].DestBlendAlpha = D3D12_BLEND_ONE,
            |d| d.blend_state.RenderTarget[7].BlendOpAlpha = D3D12_BLEND_OP_ADD,
            |d| d.blend_state.RenderTarget[7].LogicOp = D3D12_LOGIC_OP_OR,
            |d| d.blend_state.RenderTarget[7].RenderTargetWriteMask = 8,
            |d| d.rasterizer_state.FillMode = D3D12_FILL_MODE_SOLID,
            |d| d.rasterizer_state.CullMode = D3D12_CULL_MODE_BACK,
            |d| d.rasterizer_state.FrontCounterClockwise = true.into(),
            |d| d.rasterizer_state.DepthBias = -1,
            |d| d.rasterizer_state.DepthBiasClamp = -0.,
            |d| d.rasterizer_state.SlopeScaledDepthBias = 0.5,
            |d| d.rasterizer_state.DepthClipEnable = true.into(),
            |d| d.rasterizer_state.MultisampleEnable = true.into(),
            |d| d.rasterizer_state.AntialiasedLineEnable = true.into(),
            |d| d.rasterizer_state.ForcedSampleCount = 4,
            |d| d.rasterizer_state.ConservativeRaster = D3D12_CONSERVATIVE_RASTERIZATION_MODE_ON,
            |d| d.depth_stencil_state.DepthEnable = true.into(),
            |d| d.depth_stencil_state.DepthWriteMask = D3D12_DEPTH_WRITE_MASK_ALL,
            |d| d.depth_stencil_state.DepthFunc = D3D12_COMPARISON_FUNC_LESS,
            |d| d.depth_stencil_state.StencilEnable = true.into(),
            |d| d.depth_stencil_state.StencilReadMask = 1,
            |d| d.depth_stencil_state.StencilWriteMask = 1,
            |d| d.depth_stencil_state.FrontFace.StencilFailOp = D3D12_STENCIL_OP_KEEP,
            |d| d.depth_stencil_state.FrontFace.StencilDepthFailOp = D3D12_STENCIL_OP_KEEP,
            |d| d.depth_stencil_state.FrontFace.StencilPassOp = D3D12_STENCIL_OP_KEEP,
            |d| d.depth_stencil_state.FrontFace.StencilFunc = D3D12_COMPARISON_FUNC_ALWAYS,
            |d| d.depth_stencil_state.BackFace.StencilFailOp = D3D12_STENCIL_OP_KEEP,
            |d| d.depth_stencil_state.BackFace.StencilDepthFailOp = D3D12_STENCIL_OP_KEEP,
            |d| d.depth_stencil_state.BackFace.StencilPassOp = D3D12_STENCIL_OP_KEEP,
            |d| d.depth_stencil_state.BackFace.StencilFunc = D3D12_COMPARISON_FUNC_ALWAYS,
            |d| d.primitive_topology_type = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            |d| d.rtv_formats.NumRenderTargets = 1,
            |d| d.rtv_formats.RTFormats[7] = DXGI_FORMAT_R8G8B8A8_UNORM,
            |d| d.dsv_format = DXGI_FORMAT_D32_FLOAT,
            |d| d.sample_desc.Count = 4,
            |d| d.sample_desc.Quality = 1,
            |d| d.node_mask = 1,
            |d| d.flags = D3D12_PIPELINE_STATE_FLAG_TOOL_DEBUG,
            |d| d.index_buffer_strip_cut_value = D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_0xFFFF,
            |d| d.stream_output.RasterizedStream = 1,
        ];
        for (index, change) in changes.iter().enumerate() {
            let mut changed = descriptor();
            change(&mut changed);
            assert_ne!(
                unsafe { render(&changed, &[1; 32]) },
                original,
                "native field {index}"
            );
        }
        assert_ne!(unsafe { render(&descriptor(), &[2; 32]) }, original);
    }

    #[test]
    fn payload_addresses_do_not_select_name_but_shader_semantics_layout_and_output_do() {
        let first = alloc::vec![1_u8, 2, 3, 4];
        let second = first.clone();
        assert_ne!(first.as_ptr(), second.as_ptr());
        let mut d = descriptor();
        d.vertex_shader = D3D12_SHADER_BYTECODE {
            pShaderBytecode: first.as_ptr().cast(),
            BytecodeLength: first.len(),
        };
        let key = unsafe { render(&d, &[1; 32]) };
        d.vertex_shader.pShaderBytecode = second.as_ptr().cast();
        assert_eq!(unsafe { render(&d, &[1; 32]) }, key);
        let changed = [1_u8, 2, 3, 5];
        for target in 0..4 {
            let mut d = descriptor();
            let shader = [
                &mut d.vertex_shader,
                &mut d.pixel_shader,
                &mut d.task_shader,
                &mut d.mesh_shader,
            ][target];
            *shader = D3D12_SHADER_BYTECODE {
                pShaderBytecode: changed.as_ptr().cast(),
                BytecodeLength: 4,
            };
            assert_ne!(unsafe { render(&d, &[1; 32]) }, unsafe {
                render(&descriptor(), &[1; 32])
            });
        }
        let mut element = D3D12_INPUT_ELEMENT_DESC {
            SemanticName: windows_core::PCSTR(c"POSITION".as_ptr().cast()),
            ..Default::default()
        };
        d.input_layout = D3D12_INPUT_LAYOUT_DESC {
            pInputElementDescs: &element,
            NumElements: 1,
        };
        let key = unsafe { render(&d, &[1; 32]) };
        element.SemanticIndex = 1;
        assert_ne!(unsafe { render(&d, &[1; 32]) }, key);
        let mut output = D3D12_SO_DECLARATION_ENTRY::default();
        d.stream_output = D3D12_STREAM_OUTPUT_DESC {
            pSODeclaration: &output,
            NumEntries: 1,
            ..Default::default()
        };
        let gap = unsafe { render(&d, &[1; 32]) };
        output.SemanticName = windows_core::PCSTR(c"".as_ptr().cast());
        assert_ne!(
            unsafe { render(&d, &[1; 32]) },
            gap,
            "null SO gap is not an empty semantic"
        );
    }

    #[test]
    fn compute_keys_bind_actual_shader_root_node_flags_and_cached_blob() {
        let bytes = [1_u8, 2];
        let mut d = D3D12_COMPUTE_PIPELINE_STATE_DESC::default();
        let original = unsafe { compute(&d, &[1; 32]) };
        d.CS = D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytes.as_ptr().cast(),
            BytecodeLength: 2,
        };
        assert_ne!(unsafe { compute(&d, &[1; 32]) }, original);
        let key = unsafe { compute(&d, &[1; 32]) };
        assert_ne!(unsafe { compute(&d, &[2; 32]) }, key);
        d.NodeMask = 1;
        assert_ne!(unsafe { compute(&d, &[1; 32]) }, key);
        d.NodeMask = 0;
        d.Flags = D3D12_PIPELINE_STATE_FLAG_TOOL_DEBUG;
        assert_ne!(unsafe { compute(&d, &[1; 32]) }, key);
        d.Flags = D3D12_PIPELINE_STATE_FLAG_NONE;
        d.CachedPSO = D3D12_CACHED_PIPELINE_STATE {
            pCachedBlob: bytes.as_ptr().cast(),
            CachedBlobSizeInBytes: 2,
        };
        assert_ne!(unsafe { compute(&d, &[1; 32]) }, key);
    }
}
