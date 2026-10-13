const MANIFEST_CONTRACT: &str = "incant-cluster-v1;naga=29.0.4;wgpu=29.0.4;group64x1x1;shared=atomic4+128x4;msl=2.4,static-threadgroup,sizes4;spv=1.0;hlsl=6.0,grid-b0,light-t0,counts-u0,masks-u1,sizes-b1,special-b2;bounds=restrict;loops=bounded";
const SOURCE_HASH: [u8;32] = [70, 22, 6, 237, 246, 217, 203, 76, 137, 88, 30, 243, 12, 207, 58, 229, 87, 10, 30, 27, 192, 20, 47, 254, 73, 83, 233, 29, 252, 44, 4, 57];
const PRODUCER_HASH: [u8;32] = [161, 253, 26, 81, 204, 34, 202, 76, 65, 50, 57, 16, 166, 202, 74, 165, 152, 26, 89, 164, 233, 228, 76, 165, 232, 12, 110, 228, 57, 101, 181, 237];
const ARTIFACTS: [Artifact;3] = [
Artifact { backend: wgpu::Backend::Metal, stage: Stage::Compute, entry: "build", hash: [29, 80, 28, 157, 171, 172, 54, 183, 83, 14, 236, 65, 3, 228, 136, 218, 26, 24, 232, 150, 73, 255, 212, 240, 90, 116, 119, 30, 27, 227, 177, 45], bytes: include_bytes!("Metal-Compute.shader") },
Artifact { backend: wgpu::Backend::Vulkan, stage: Stage::Compute, entry: "build", hash: [22, 151, 29, 26, 219, 55, 121, 178, 187, 231, 187, 180, 36, 39, 157, 219, 54, 160, 39, 166, 211, 108, 77, 18, 65, 230, 224, 36, 198, 157, 251, 18], bytes: include_bytes!("Vulkan-Compute.shader") },
Artifact { backend: wgpu::Backend::Dx12, stage: Stage::Compute, entry: "build", hash: [231, 228, 64, 30, 103, 237, 245, 6, 76, 84, 203, 132, 55, 103, 220, 44, 223, 248, 69, 44, 56, 236, 200, 125, 171, 197, 127, 230, 102, 85, 152, 179], bytes: include_bytes!("Dx12-Compute.shader") },
];
