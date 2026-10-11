//! 2x / 4x polyphase oversampling around a memoryless nonlinearity (SPEC v2 1.2; SQ-11, 13).
//!
//! Up: equiripple half-band FIRs, run polyphase (the odd phase of a half-band is its centre tap,
//! a pure delay). Down: a 63-tap equiripple low-pass at 2x (pass 19.5 kHz, stop 24 kHz at a 48 kHz
//! base rate, -64 dB) so nothing between 24 and 28 kHz folds back into the top octave, plus an
//! 11-tap half-band for the 4x -> 2x stage. Coefficients were designed with `scipy.signal.remez`
//! (half-band trick: G(z^2) of length 2K plus a 0.5 centre tap).
//!
//! Every filter is linear phase, so the latency is constant: [`latency`] base-rate samples
//! (0 / 21 / 23 for 1x / 2x / 4x). Band edges scale with the sample rate.
#![allow(clippy::excessive_precision)] // coefficients pasted at full design precision

/// Even phase of the 23-tap 2x up half-band (pass 20 k / stop 28 k at 96 kHz), gain 2 applied.
/// Odd phase = centre tap: the input delayed by `UP1_DELAY`.
const UP1_EVEN: [f32; 12] = [
    -2.300635932e-02, 3.364194112e-02, -5.916963526e-02, 1.029488932e-01, -1.968465685e-01, 6.313716931e-01,
    6.313716931e-01, -1.968465685e-01, 1.029488932e-01, -5.916963526e-02, 3.364194112e-02, -2.300635932e-02,
];
const UP1_DELAY: usize = 5;
/// Even phase of the 7-tap 4x up half-band (pass 20 k / stop 76 k at 192 kHz), gain 2 applied.
const UP2_EVEN: [f32; 4] = [-8.696365639e-02, 5.819915308e-01, 5.819915308e-01, -8.696365639e-02];
const UP2_DELAY: usize = 1;
/// 11-tap 4x -> 2x half-band (pass 24 k / stop 72 k at 192 kHz).
const DOWN1: [f32; 11] = [
    1.304920555e-02, 0.0, -6.387151210e-02, 0.0, 3.016129481e-01, 5.0e-01, 3.016129481e-01, 0.0, -6.387151210e-02,
    0.0, 1.304920555e-02,
];
/// 63-tap 2x -> 1x low-pass (pass 19.5 k / stop 24 k at 96 kHz).
const DOWN2: [f32; 63] = [
    -8.106585255e-04, -3.775735072e-04, 1.578883578e-03, 2.771858762e-03, 6.211201855e-04, -2.430330579e-03,
    -1.207657429e-03, 3.245370503e-03, 3.251452622e-03, -2.926874406e-03, -5.510799999e-03, 1.701939157e-03,
    8.003720604e-03, 9.693741207e-04, -1.006758929e-02, -5.203028367e-03, 1.103251252e-02, 1.101180362e-02,
    -1.003005104e-02, -1.814091228e-02, 6.072000797e-03, 2.610154818e-02, 2.058454908e-03, -3.421049201e-02,
    -1.624939835e-02, 4.167775272e-02, 4.081979238e-02, -4.771189715e-02, -9.155235262e-02, 5.163560593e-02,
    3.133131818e-01, 4.470047625e-01, 3.133131818e-01, 5.163560593e-02, -9.155235262e-02, -4.771189715e-02,
    4.081979238e-02, 4.167775272e-02, -1.624939835e-02, -3.421049201e-02, 2.058454908e-03, 2.610154818e-02,
    6.072000797e-03, -1.814091228e-02, -1.003005104e-02, 1.101180362e-02, 1.103251252e-02, -5.203028367e-03,
    -1.006758929e-02, 9.693741207e-04, 8.003720604e-03, 1.701939157e-03, -5.510799999e-03, -2.926874406e-03,
    3.251452622e-03, 3.245370503e-03, -1.207657429e-03, -2.430330579e-03, 6.211201855e-04, 2.771858762e-03,
    1.578883578e-03, -3.775735072e-04, -8.106585255e-04,
];

/// Base-rate latency in samples for an oversampling factor (1, 2 or 4).
/// 2x: (UP1 centre 11 + DOWN2 centre 31) / 2 = 21. 4x adds (UP2 centre 3 + DOWN1 centre 5) / 4 = 2.
pub const fn latency(factor: usize) -> usize {
    match factor {
        2 => 21,
        4 => 23,
        _ => 0,
    }
}

/// Sample history kept twice over so the last `N` samples are always one contiguous slice.
#[derive(Clone, Copy)]
struct Hist<const N: usize> {
    buf: [[f32; N]; 2],
    w: usize,
}

impl<const N: usize> Hist<N> {
    const ZERO: Self = Self { buf: [[0.0; N]; 2], w: 0 };

    #[inline]
    fn push(&mut self, x: f32) {
        // Slot w of both copies, read as one 2N-long array: window = flat[w+1 ..= w+N].
        self.buf[0][self.w] = x;
        self.buf[1][self.w] = x;
        self.w = (self.w + 1) % N;
    }

    /// Dot product of the last N samples with `taps` (taps[0] applies to the newest sample).
    #[inline]
    fn dot(&self, taps: &[f32; N]) -> f32 {
        let flat = self.buf.as_flattened();
        let win = &flat[self.w..self.w + N]; // oldest .. newest
        win.iter().zip(taps.iter().rev()).map(|(a, b)| a * b).sum()
    }

    /// The sample pushed `d` pushes ago (0 = newest).
    #[inline]
    fn back(&self, d: usize) -> f32 {
        self.buf[0][(self.w + N - 1 - d) % N]
    }
}

/// Two cascaded 2x stages; `factor` 2 uses only the outer one.
#[derive(Clone, Copy)]
pub struct Oversampler {
    up1: Hist<12>,
    up2: Hist<4>,
    down1: Hist<11>,
    down2: Hist<63>,
}

impl Default for Oversampler {
    fn default() -> Self {
        Self { up1: Hist::ZERO, up2: Hist::ZERO, down1: Hist::ZERO, down2: Hist::ZERO }
    }
}

impl Oversampler {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    #[inline]
    fn up(h: &mut Hist<12>, x: f32) -> [f32; 2] {
        h.push(x);
        [h.dot(&UP1_EVEN), h.back(UP1_DELAY)]
    }

    /// One base-rate sample through `f` at `factor` x (2 or 4; anything else runs `f` at 1x).
    #[inline]
    pub fn process(&mut self, x: f32, factor: usize, f: impl Fn(f32) -> f32) -> f32 {
        let [a, b] = Self::up(&mut self.up1, x);
        let pair = match factor {
            2 => [f(a), f(b)],
            4 => {
                let mut out = [0.0; 2];
                for (o, v) in out.iter_mut().zip([a, b]) {
                    self.up2.push(v);
                    let (p, q) = (f(self.up2.dot(&UP2_EVEN)), f(self.up2.back(UP2_DELAY)));
                    // Decimate on the even sample, then keep the odd one as history.
                    self.down1.push(p);
                    *o = self.down1.dot(&DOWN1);
                    self.down1.push(q);
                }
                out
            }
            _ => return f(x),
        };
        self.down2.push(pair[0]);
        let y = self.down2.dot(&DOWN2);
        self.down2.push(pair[1]);
        y
    }
}
