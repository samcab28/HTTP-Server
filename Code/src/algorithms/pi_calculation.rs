//! Pi calculation algorithms
//!
//! Implements a decimal spigot algorithm (Rabinowitz–Wagon) to generate
//! D digits of pi without big integer libraries. Suitable for a few
//! thousand digits; for much larger D, consider Chudnovsky with bignums.

/// Generate a string representation of pi with exactly `digits` digits
/// after the decimal point using a spigot algorithm. Returns "3" when
/// `digits == 0`, otherwise "3.<digits>".
pub fn pi_spigot_string(digits: u32) -> String {
    let d = digits as usize;
    if d == 0 {
        return "3".to_string();
    }
    // Number of boxes; classic formula
    let len = d * 10 / 3 + 1;
    let mut a = vec![2u32; len];
    let mut nines: usize = 0;
    let mut predigit: u8 = 0;
    let mut have_predigit = false;
    let mut digits_out: Vec<u8> = Vec::with_capacity(d + 1);

    for _ in 0..(d+1) {
        let mut carry: u32 = 0;
        // Work backwards over boxes 1..len-1
        for j in (1..len).rev() {
            let j_u = j as u32;
            let x = a[j] * 10 + carry;
            let q = x / (2 * j_u + 1);
            a[j] = x % (2 * j_u + 1);
            carry = q * j_u;
        }
        let x = a[0] * 10 + carry;
        let q = (x / 10) as u8; // 0..10
        a[0] = x % 10;

        if q == 9 {
            nines += 1;
        } else if q == 10 {
            // We need to increment previous printed digit
            if have_predigit {
                digits_out.push(predigit + 1);
            } else {
                // First digit case: becomes 1
                digits_out.push(1);
                have_predigit = true;
            }
            digits_out.extend(std::iter::repeat(0).take(nines));
            predigit = 0;
            nines = 0;
        } else {
            if have_predigit {
                digits_out.push(predigit);
            } else {
                have_predigit = true;
            }
            predigit = q;
            digits_out.extend(std::iter::repeat(9).take(nines));
            nines = 0;
        }
    }
    // append last predigit
    digits_out.push(predigit);

    // Build string: first digit is integer part '3'
    let int_part = digits_out[0];
    let frac = &digits_out[1..]; // exactly d digits
    let mut s = String::with_capacity(2 + d);
    s.push((b'0' + int_part) as char);
    s.push('.');
    for &dg in frac { s.push((b'0' + dg) as char); }
    s
}


/// Compute pi using the Chudnovsky series with f64 arithmetic and
/// return a decimal string with `digits` digits after the decimal point.
/// Note: f64 limits usable precision to about 15–16 digits.
pub fn pi_chudnovsky_string(digits: u32) -> String {
    // Number of series terms needed grows about digits/14
    let terms = ((digits as f64) / 14.0).ceil().max(1.0) as u32;

    let mut sum = 0.0f64;
    let mut m = 1.0f64; // M_k ratio accumulator
    let mut l = 13591409.0f64; // L_k = 13591409 + 545140134 k
    let x = 640320.0f64;
    let mut xk = 1.0f64; // x^{3k}
    let mut sign = 1.0f64;

    for k in 0..terms {
        // term_k = (-1)^k * M_k * L_k / X^{3k}
        let term = sign * m * l / xk;
        sum += term;

        // Prepare next
        let kf = (k as f64) + 1.0;
        // M_{k+1}/M_k ratio
        let num = (6.0 * kf - 5.0)
            * (6.0 * kf - 4.0)
            * (6.0 * kf - 3.0)
            * (6.0 * kf - 2.0)
            * (6.0 * kf - 1.0)
            * (6.0 * kf);
        let den = (3.0 * kf - 2.0) * (3.0 * kf - 1.0) * (3.0 * kf) * (kf * kf * kf);
        m *= num / den;

        l += 545140134.0;
        xk *= x * x * x;
        sign = -sign;
    }

    let pi = (426880.0f64 * 10005.0f64.sqrt()) / sum;
    // Format with rounding to `digits`
    format!("{:.*}", digits as usize, pi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_digits() {
        let s = pi_spigot_string(10);
        assert!(s.starts_with("3.14159"), "got {}", s);
    }
}
