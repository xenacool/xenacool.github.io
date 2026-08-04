pub fn compute_payload_hash(payload: &[u8]) -> u32 {
    let mut hash = 0u32;
    for chunk in payload.chunks(4) {
        let mut b = [0u8; 4];
        for (i, &v) in chunk.iter().enumerate() {
            b[i] = v;
        }
        hash ^= u32::from_le_bytes(b);
    }
    hash
}
