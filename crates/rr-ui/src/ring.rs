//! Lock-free single-producer / single-consumer sample ring for oscilloscopes and meters.
//!
//! The audio thread [`RingWriter::push`]es samples (wait-free, no allocation, no lock); the GUI
//! thread calls [`RingReader::latest`] ~60 times a second to copy the most recent N samples.
//! Overwrite semantics: the writer never waits for the reader, the reader only ever wants "the
//! last N samples", so a slow GUI simply skips audio it never needed.
//!
//! Every slot is an `AtomicU32` holding the f32 bits, so there is no `unsafe` and no data race.
//! A reader that was lapped mid-copy detects it (the write counter moved more than the spare
//! capacity) and retries, so a returned window is always a contiguous, untorn run of samples.

use std::sync::atomic::{fence, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

struct Inner {
    slots: Box<[AtomicU32]>,
    mask: u64,
    /// Total samples ever written (monotonic). Release-stored after the samples themselves.
    written: AtomicU64,
    /// End of the push in progress, stored (then a Release fence) before its slots: a reader that
    /// copied an overwritten slot sees it after its Acquire fence (the seqlock pattern, as in snap).
    claimed: AtomicU64,
}

/// Creates a ring of `capacity` samples (rounded up to a power of two, at least 2).
pub fn ring(capacity: usize) -> (RingWriter, RingReader) {
    let cap = capacity.max(2).next_power_of_two();
    let inner = Arc::new(Inner { slots: (0..cap).map(|_| AtomicU32::new(0)).collect(), mask: cap as u64 - 1, written: AtomicU64::new(0), claimed: AtomicU64::new(0) });
    (RingWriter { inner: inner.clone(), pos: 0 }, RingReader { inner })
}

pub struct RingWriter {
    inner: Arc<Inner>,
    pos: u64,
}

impl RingWriter {
    /// Audio thread: wait-free, allocation-free.
    pub fn push(&mut self, samples: &[f32]) {
        self.inner.claimed.store(self.pos + samples.len() as u64, Ordering::Relaxed);
        fence(Ordering::Release);
        for &s in samples {
            self.inner.slots[(self.pos & self.inner.mask) as usize].store(s.to_bits(), Ordering::Relaxed);
            self.pos += 1;
        }
        self.inner.written.store(self.pos, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct RingReader {
    inner: Arc<Inner>,
}

impl RingReader {
    pub fn capacity(&self) -> usize {
        self.inner.slots.len()
    }

    /// Total samples written so far.
    pub fn written(&self) -> u64 {
        self.inner.written.load(Ordering::Acquire)
    }

    /// Fills `out` with the most recent `out.len()` samples, oldest first. Returns `false` (and
    /// leaves `out` unspecified) if fewer than `out.len()` samples exist yet or the window is
    /// larger than half the capacity (the writer could lap the copy every time).
    pub fn latest(&self, out: &mut [f32]) -> bool {
        self.window(out, None)
    }

    /// Like [`latest`](Self::latest) but the window ends at sample count `end` (a value of
    /// [`written`](Self::written), possibly of another ring pushed in step with this one), so two
    /// rings can be read sample-aligned. `false` when `end` is in the future or already overwritten.
    pub fn latest_ending(&self, out: &mut [f32], end: u64) -> bool {
        self.window(out, Some(end))
    }

    fn window(&self, out: &mut [f32], at: Option<u64>) -> bool {
        let n = out.len() as u64;
        if n > self.inner.mask.div_ceil(2) {
            return false;
        }
        for _ in 0..8 {
            let written = self.inner.written.load(Ordering::Acquire);
            let end = at.unwrap_or(written);
            if end < n || end > written {
                return false;
            }
            let start = end - n;
            for (i, o) in out.iter_mut().enumerate() {
                *o = f32::from_bits(self.inner.slots[((start + i as u64) & self.inner.mask) as usize].load(Ordering::Relaxed));
            }
            // Slots older than `end2 - cap` may have been overwritten while we copied (including
            // by a push still in progress).
            fence(Ordering::Acquire);
            let end2 = self.inner.claimed.load(Ordering::Relaxed);
            if end2 - start <= self.inner.mask + 1 {
                return true;
            }
        }
        false
    }
}

/// Index of the first rising zero crossing in `w`, for a steady scope trace.
pub fn rising_zero(w: &[f32]) -> Option<usize> {
    w.windows(2).position(|p| p[0] < 0.0 && p[1] >= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_returns_last_samples_oldest_first_across_wraparound() {
        let (mut w, r) = ring(16);
        let mut out = [0.0; 4];
        assert!(!r.latest(&mut out), "empty ring has no window");
        for chunk in (0..50).map(|i| i as f32).collect::<Vec<_>>().chunks(7) {
            w.push(chunk);
        }
        assert!(r.latest(&mut out));
        assert_eq!(out, [46.0, 47.0, 48.0, 49.0]);
        assert_eq!(r.written(), 50);
        assert!(!r.latest(&mut [0.0; 9]), "window over half the capacity is refused");
        assert!(r.latest_ending(&mut out, 40));
        assert_eq!(out, [36.0, 37.0, 38.0, 39.0]);
        assert!(!r.latest_ending(&mut out, 51), "future end");
        assert!(!r.latest_ending(&mut out, 20), "overwritten");
    }

    #[test]
    fn rising_zero_finds_the_crossing() {
        assert_eq!(rising_zero(&[0.5, -0.2, -0.1, 0.3, 0.9]), Some(2));
        assert_eq!(rising_zero(&[0.5, 0.2]), None);
    }

    /// Writer ramps 0,1,2,... as fast as it can; every window the reader gets must be consecutive.
    #[test]
    fn concurrent_reader_never_sees_a_torn_window() {
        let (mut w, r) = ring(4096);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let s2 = stop.clone();
        let t = std::thread::spawn(move || {
            let mut v = 0.0f32;
            let mut buf = [0.0f32; 64];
            while !s2.load(Ordering::Relaxed) {
                for b in buf.iter_mut() {
                    *b = v;
                    v += 1.0;
                }
                w.push(&buf);
            }
        });
        let mut out = [0.0f32; 2048];
        let (mut ok, mut skipped) = (0, 0);
        let t0 = std::time::Instant::now();
        while t0.elapsed() < std::time::Duration::from_millis(400) {
            if r.latest(&mut out) {
                ok += 1;
                assert!(out.windows(2).all(|p| p[1] == p[0] + 1.0), "torn window");
            } else {
                skipped += 1;
            }
        }
        stop.store(true, Ordering::Relaxed);
        t.join().unwrap();
        assert!(ok > 100, "only {ok} reads succeeded ({skipped} refused)");
    }
}
