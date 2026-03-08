//! Primality testing and factorization (trial division)
//!
//! Stage 1 implements deterministic trial division up to sqrt(n).
//! Miller–Rabin will be added in a later stage.

/// Return true if `n` is prime using trial division.
/// Handles small cases and divides by 2, then odd divisors up to sqrt(n).
pub fn is_prime_trial(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n.is_multiple_of(2) {
        return false;
    }

    // Check odd divisors up to sqrt(n)
    let mut d: u64 = 3;
    while d.saturating_mul(d) <= n {
        if n.is_multiple_of(d) {
            return false;
        }
        d += 2;
    }
    true
}

/// Fast modular exponentiation: a^e mod m
fn mod_pow(mut a: u128, mut e: u128, m: u128) -> u128 {
    let mut result: u128 = 1 % m;
    a %= m;
    while e > 0 {
        if e & 1 == 1 { result = (result * a) % m; }
        a = (a * a) % m;
        e >>= 1;
    }
    result
}

/// Miller–Rabin witness test for base `a` and odd n > 2
fn mr_witness(a: u64, n: u64, d: u64, s: u32) -> bool {
    let n128 = n as u128;
    let mut x = mod_pow(a as u128, d as u128, n128) as u64;
    if x == 1 || x == n - 1 { return false; }
    let mut i = 1;
    while i < s {
        x = ((x as u128 * x as u128) % n128) as u64;
        if x == n - 1 { return false; }
        i += 1;
    }
    true
}

/// Deterministic bases to cover 64-bit range.
const MR_BASES_64: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

/// Miller–Rabin probabilistic primality test (64-bit), using fixed small bases.
/// Rounds selects how many bases to try (up to MR_BASES_64.len()).
pub fn is_prime_mr(n: u64, rounds: u32) -> bool {
    if n < 2 { return false; }
    // Handle small primes and even
    for &p in [2u64,3,5,7,11,13,17,19,23,29,31,37].iter() {
        if n == p { return true; }
        if n.is_multiple_of(p) { return n == p; }
    }
    // write n-1 = d * 2^s with d odd
    let mut d = n - 1;
    let mut s: u32 = 0;
    while d.is_multiple_of(2) { d /= 2; s += 1; }

    let r = rounds.min(MR_BASES_64.len() as u32) as usize;
    for &a in MR_BASES_64.iter().take(r) {
        if a % n == 0 { continue; }
        if mr_witness(a, n, d, s) { return false; }
    }
    true
}

/// Factorize `n` into prime powers using trial division.
/// Returns a vector of (prime, count) with primes in ascending order.
pub fn factor_trial(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors: Vec<(u64, u32)> = Vec::new();
    if n < 2 {
        return factors;
    }

    // Factor out powers of 2
    let mut count: u32 = 0;
    while n.is_multiple_of(2) {
        n /= 2;
        count += 1;
    }
    if count > 0 {
        factors.push((2, count));
    }

    // Factor odd divisors
    let mut d: u64 = 3;
    while d.saturating_mul(d) <= n {
        if n.is_multiple_of(d) {
            let mut c: u32 = 0;
            while n.is_multiple_of(d) {
                n /= d;
                c += 1;
            }
            factors.push((d, c));
        }
        d += 2;
    }

    if n > 1 {
        // Remaining prime factor
        factors.push((n, 1));
    }

    factors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_prime_basic() {
        let primes = [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31];
        for &p in &primes {
            assert!(is_prime_trial(p), "{} should be prime", p);
        }
        let comps = [0u64, 1, 4, 6, 8, 9, 10, 12, 15, 21, 25, 27];
        for &c in &comps {
            assert!(!is_prime_trial(c), "{} should be composite", c);
        }
    }

    #[test]
    fn test_factor_trial_small() {
        assert!(factor_trial(1).is_empty());
        assert_eq!(factor_trial(2), vec![(2, 1)]);
        assert_eq!(factor_trial(3), vec![(3, 1)]);
        assert_eq!(factor_trial(4), vec![(2, 2)]);
        assert_eq!(factor_trial(360), vec![(2, 3), (3, 2), (5, 1)]);
        assert_eq!(factor_trial(1024), vec![(2, 10)]);
    }

    #[test]
    fn test_mr_basic() {
        // Some known primes and composites
        let primes = [2u64, 3, 5, 7, 11, 13, 97, 9973, 104729, 982451653];
        for &p in &primes {
            assert!(is_prime_mr(p, 6), "{} should be prime (mr)", p);
        }
        let comps = [1u64, 4, 6, 8, 9, 10, 12, 15, 21, 25, 27, 341, 561, 1105];
        for &c in &comps {
            assert!(!is_prime_mr(c, 6), "{} should be composite (mr)", c);
        }
    }
}
