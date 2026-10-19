//! Stable name colours shared by view owners and surfaces.
/// FNV-1a over the exact UTF-8 name, matching the client identity fixture.
pub fn hash(name: &str) -> u32 {
    name.bytes().fold(2_166_136_261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    })
}
pub fn hue(hash: u32) -> usize {
    ((hash >> 8) % 5) as usize
}
pub fn ink(name: &str) -> &'static str {
    ["crew0", "crew1", "crew2", "crew3", "crew4"][hue(hash(name))]
}
