//! Lock-free single-writer snapshot of a fixed number of f32 values (a seqlock over atomics).
//!
//! The audio thread [`Snap::write`]s a whole record once per block (wait-free, no allocation);
//! the GUI thread [`Snap::read`]s it ~60 times a second. A read that overlapped a write is
//! detected by the sequence counter and retried, so a successful read is always one consistent
//! record, never a mix of two blocks. Every slot is an `AtomicU32` holding f32 bits: no `unsafe`,
//! no data race. Only ONE thread may write.

use std::sync::atomic::{fence, AtomicU32, Ordering};

pub struct Snap {
    /// Odd while a write is in progress; bumped by 2 per completed write.
    seq: AtomicU32,
    slots: Box<[AtomicU32]>,
}

impl Snap {
    /// A record of `n` values, all 0.
    pub fn new(n: usize) -> Self {
        Self { seq: AtomicU32::new(0), slots: (0..n).map(|_| AtomicU32::new(0)).collect() }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Completed writes so far.
    pub fn version(&self) -> u32 {
        self.seq.load(Ordering::Acquire) / 2
    }

    /// Writer thread only (single writer). Wait-free, allocation-free. Extra values are ignored,
    /// missing ones keep their previous value.
    pub fn write(&self, v: &[f32]) {
        let s = self.seq.load(Ordering::Relaxed);
        self.seq.store(s.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        for (slot, x) in self.slots.iter().zip(v) {
            slot.store(x.to_bits(), Ordering::Relaxed);
        }
        self.seq.store(s.wrapping_add(2), Ordering::Release);
    }

    /// Copies the latest complete record into `out` (up to `out.len()` values). Returns `false`
    /// (leaving `out` unspecified) only if every retry raced a write, which needs a writer that
    /// rewrites continuously; the caller then keeps its previous copy.
    pub fn read(&self, out: &mut [f32]) -> bool {
        for _ in 0..32 {
            let s1 = self.seq.load(Ordering::Acquire);
            if s1 & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            for (o, slot) in out.iter_mut().zip(self.slots.iter()) {
                *o = f32::from_bits(slot.load(Ordering::Relaxed));
            }
            fence(Ordering::Acquire);
            if self.seq.load(Ordering::Relaxed) == s1 {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn write_then_read_round_trips() {
        let s = Snap::new(4);
        let mut out = [9.0; 4];
        assert!(s.read(&mut out));
        assert_eq!(out, [0.0; 4]);
        s.write(&[1.0, 2.0, 3.0, 4.0]);
        assert!(s.read(&mut out));
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(s.version(), 1);
    }

    /// The writer stores the same counter in every slot as fast as it can; a reader must never
    /// see two different values in one record (a torn snapshot).
    #[test]
    fn concurrent_reader_never_sees_a_torn_record() {
        const N: usize = 136;
        let s = Arc::new(Snap::new(N));
        let stop = Arc::new(AtomicBool::new(false));
        let (w, st) = (s.clone(), stop.clone());
        let t = std::thread::spawn(move || {
            let mut rec = [0.0f32; N];
            let mut k = 0u32;
            while !st.load(Ordering::Relaxed) {
                k += 1;
                rec.fill(f32::from_bits(k)); // bit patterns, so the count never rounds
                w.write(&rec);
            }
            k
        });
        let mut out = [0.0f32; N];
        let (mut ok, mut last) = (0u32, 0u32);
        let t0 = std::time::Instant::now();
        while t0.elapsed() < std::time::Duration::from_millis(400) {
            if s.read(&mut out) {
                ok += 1;
                let k = out[0].to_bits();
                assert!(out.iter().all(|v| v.to_bits() == k), "torn record: {k} .. {}", out[N - 1].to_bits());
                assert!(k >= last, "went back in time");
                last = k;
            }
        }
        stop.store(true, Ordering::Relaxed);
        let writes = t.join().unwrap();
        assert!(ok > 100 && writes > 1000, "{ok} reads, {writes} writes");
    }
}
