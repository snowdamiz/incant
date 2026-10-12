use incant_runtime::AnimationGraphSnapshot;
use incant_wasm_abi as abi;
pub(crate) fn encode(rows: &[AnimationGraphSnapshot], graph: &[u8], bytes: &mut Vec<u8>) {
    bytes.clear();
    bytes.extend(
        abi::AnimationGraphHeaderV2 {
            magic: *abi::ANIMATION_GRAPH_V2_MAGIC,
            graph_bytes: graph.len() as u32,
            count: rows.iter().filter(|r| r.fade.is_some()).count() as u32,
            stride: abi::AnimationGraphFade::SIZE as u32,
            reserved: 0,
        }
        .encode(),
    );
    bytes.extend(graph);
    for row in rows {
        if let Some(f) = row.fade {
            bytes.extend(
                abi::AnimationGraphFade {
                    entity: row.entity.0,
                    state: f.state.0,
                    seconds: f.seconds,
                    speed: f.speed,
                    elapsed: f.elapsed,
                    duration: f.duration,
                    flags: u32::from(f.playing),
                    reserved: 0,
                }
                .encode(),
            );
        }
    }
    debug_assert!(bytes.len() <= abi::ANIMATION_GRAPH_V2_MAX_BYTES);
}
