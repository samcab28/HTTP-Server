//! Mandelbrot iteration map generator

/// Generate a 2D iteration count map for the Mandelbrot set.
/// Maps pixel grid [0,width) x [0,height) onto the complex plane
/// x in [-2.5, 1.0], y in [-1.5, 1.5].
pub fn mandelbrot_iterations(width: u32, height: u32, max_iter: u32) -> Vec<Vec<u32>> {
    let w = width.max(1) as usize;
    let h = height.max(1) as usize;
    let max_iter = max_iter.max(1);

    let mut rows: Vec<Vec<u32>> = Vec::with_capacity(h);
    let x_min = -2.5f64;
    let x_max = 1.0f64;
    let y_min = -1.5f64;
    let y_max = 1.5f64;

    for j in 0..h {
        let mut row = Vec::with_capacity(w);
        let cy = y_min + (y_max - y_min) * (j as f64) / ((h - 1).max(1) as f64);
        for i in 0..w {
            let cx = x_min + (x_max - x_min) * (i as f64) / ((w - 1).max(1) as f64);
            let mut x = 0.0f64;
            let mut y = 0.0f64;
            let mut iter = 0u32;
            while x * x + y * y <= 4.0 && iter < max_iter {
                let xt = x * x - y * y + cx;
                y = 2.0 * x * y + cy;
                x = xt;
                iter += 1;
            }
            row.push(iter);
        }
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiny_map() {
        let m = mandelbrot_iterations(3, 3, 10);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].len(), 3);
    }
}
