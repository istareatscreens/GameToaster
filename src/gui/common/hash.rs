pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/gui/common/hash__tests.rs"]
mod tests;
