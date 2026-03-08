//! Matrix operations used by CPU-bound endpoints

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate two pseudo-random matrices A and B (size n x n) using an LCG
/// and compute C = A * B (naive O(n^3)). Returns a hex hash of C suitable
/// for verification. Hashing is done incrementally to avoid storing C.
pub fn matrixmul_hash(size: u32, seed: u64) -> String {
    let n = size as usize;
    let mut a = vec![0i32; n * n];
    let mut b = vec![0i32; n * n];

    // Simple LCG similar to utils::crypto
    let mut state = if seed == 0 { 0xdead_beef_cafe_babe } else { seed };
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        // Map to small range to keep products in i64 bounds
        ((state >> 33) as i32) & 0x3ff // 0..1023
    };

    for v in &mut a { *v = next(); }
    for v in &mut b { *v = next(); }

    let mut hasher = DefaultHasher::new();

    // C[i,j] = sum_k A[i,k] * B[k,j]
    for i in 0..n {
        for j in 0..n {
            let mut acc: i64 = 0;
            let row = &a[i * n..(i + 1) * n];
            for k in 0..n {
                acc += row[k] as i64 * b[k * n + j] as i64;
            }
            // Feed position and value into hasher
            (i as u32, j as u32, acc).hash(&mut hasher);
        }
    }

    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_hash() {
        let h1 = matrixmul_hash(4, 12345);
        let h2 = matrixmul_hash(4, 12345);
        assert_eq!(h1, h2);
        let h3 = matrixmul_hash(4, 54321);
        assert_ne!(h1, h3);
    }
}
