pub fn mul_div_round_up(a: u64, b: u64, c: u64) -> u64 {
    a.checked_mul(b)
        .unwrap()
        .checked_add(c - 1)
        .unwrap()
        .checked_div(c)
        .unwrap()
}
