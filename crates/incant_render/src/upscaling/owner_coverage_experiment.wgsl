// Test-only metadata. This bit is independent of rounded RGBA16 alpha. It says
// at least one positively weighted contribution was not proved fully covered;
// it does not prove a geometric area, independent samples, or a single layer.
const PARTIAL_COVERAGE:u32=0x20000u;
fn coverage_encode(owners:Owners,partial:bool)->vec2u {
    var record=encode(owners);
    if record.x!=0u && partial {record.x|=PARTIAL_COVERAGE;}
    return record;
}
