pub fn clamp_u128_to_u64(i: u128) -> u64 {
    i.min(u128::from(u64::MAX)) as u64
}
