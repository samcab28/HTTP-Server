//! Cryptographic and random number utilities

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Calculate SHA256 hash (simplified - using standard hash for now)
/// In production, use a proper crypto library like sha2
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Generate random numbers in a range
/// Using a simple PRNG for demonstration
pub fn generate_random_numbers(count: usize, min: i64, max: i64) -> Vec<i64> {
    use std::time::SystemTime;

    let mut numbers = Vec::with_capacity(count);
    let seed = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let mut state = seed;
    let range = (max - min) as u64;

    for _ in 0..count {
        // Simple LCG
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let value = min + ((state / 65536) % range) as i64;
        numbers.push(value);
    }

    numbers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_random_numbers() {
        let nums = generate_random_numbers(10, 0, 100);
        assert_eq!(nums.len(), 10);
        assert!(nums.iter().all(|&n| n >= 0 && n < 100));
    }
}