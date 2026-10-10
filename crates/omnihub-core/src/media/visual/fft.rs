//! A small in-place radix-2 FFT (power-of-two sizes), enough for a 2048-point
//! spectrum sixty times a second.

#[derive(Debug, Clone)]
pub struct Fft {
    n: usize,
    /// cos/sin of 2πk/n for k < n/2.
    twiddle: Vec<(f32, f32)>,
    /// Bit-reversed index of each position.
    rev: Vec<u32>,
}

impl Fft {
    pub fn new(n: usize) -> Fft {
        assert!(n.is_power_of_two() && n >= 2);
        let bits = n.trailing_zeros();
        let twiddle = (0..n / 2).map(|k| (std::f32::consts::TAU * k as f32 / n as f32).sin_cos()).map(|(s, c)| (c, -s)).collect();
        let rev = (0..n as u32).map(|i| i.reverse_bits() >> (32 - bits)).collect();
        Fft { n, twiddle, rev }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Transform `re`/`im` in place (forward, unscaled).
    pub fn run(&self, re: &mut [f32], im: &mut [f32]) {
        let n = self.n;
        assert!(re.len() == n && im.len() == n);
        for i in 0..n {
            let j = self.rev[i] as usize;
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let step = n / size;
            for start in (0..n).step_by(size) {
                for k in 0..half {
                    let (wr, wi) = self.twiddle[k * step];
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            size *= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_direct_transform() {
        let n = 64;
        let fft = Fft::new(n);
        let x: Vec<f32> = (0..n).map(|i| ((i * 7 % 13) as f32 - 6.0) / 6.0).collect();
        let (mut re, mut im) = (x.clone(), vec![0.0; n]);
        fft.run(&mut re, &mut im);
        for k in 0..n {
            let (mut r, mut m) = (0.0f64, 0.0f64);
            for (t, v) in x.iter().enumerate() {
                let a = -std::f64::consts::TAU * (k * t) as f64 / n as f64;
                r += *v as f64 * a.cos();
                m += *v as f64 * a.sin();
            }
            assert!((re[k] as f64 - r).abs() < 1e-3 && (im[k] as f64 - m).abs() < 1e-3, "bin {k}");
        }
    }

    #[test]
    fn a_sine_lands_in_its_bin() {
        let n = 2048;
        let fft = Fft::new(n);
        let mut re: Vec<f32> = (0..n).map(|i| (std::f32::consts::TAU * 40.0 * i as f32 / n as f32).sin()).collect();
        let mut im = vec![0.0; n];
        fft.run(&mut re, &mut im);
        let mags: Vec<f32> = (0..n / 2).map(|k| re[k].hypot(im[k])).collect();
        let peak = mags.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0;
        assert_eq!(peak, 40);
        assert!((mags[40] - n as f32 / 2.0).abs() < 1.0);
    }
}
