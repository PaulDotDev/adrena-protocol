pub fn u64_from_slice(slice: &[u8]) -> u64 {
    u64::from_ne_bytes(slice.split_at(8).0.try_into().unwrap())
}
