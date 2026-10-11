//! RR-BUS: a process-wide, lock-free, control-rate signal bus so Riddim Rack devices in one host
//! session can modulate each other. See SPEC.md "RR-BUS — inter-device modulation".
//!
//! Transport: on Windows a named page-file mapping (`Local\RiddimRackBus.v1`) shared by every
//! process in the session; elsewhere a process-local static with the same layout. Every field in
//! the mapping is an atomic. Opening never fails: anything unexpected yields a disabled [`Bus`]
//! whose operations are no-ops.
//!
//! Thread rules:
//! - Audio thread (per block): [`Source::set`], [`Subscription::read`], [`Instance::heartbeat`].
//!   These do atomic loads/stores plus one wall-clock read in `heartbeat`; no allocation, no
//!   locks, no blocking syscalls.
//! - GUI/host thread: [`Bus::open`], [`Bus::register`], [`Instance::publish`],
//!   [`Instance::set_label`], [`Bus::list`], [`Subscription::bind`],
//!   [`Subscription::rebind_if_lost`]. These may allocate and scan the tables.
//!
//! ```
//! use rr_bus::{Bus, Kind, Subscription};
//!
//! // A device calls `Bus::open()` in `initialize`; the example uses its own name so it never
//! // touches a bus a running DAW is using.
//! let bus = Bus::open_named(&format!("Local\\RiddimRackBus.doc.{}", std::process::id()));
//!
//! // Publisher, GUI thread: register once, publish once per signal.
//! let gts = bus.register("GTS"); // label defaults to "GTS 1"
//! let mod_a = gts.publish("Mod A", Kind::Bipolar);
//!
//! // Publisher, audio thread, once per block.
//! gts.heartbeat();
//! mod_a.set(-0.25);
//!
//! // Subscriber, GUI thread: bind by label + name (usually restored from "GTS 1 · Mod A").
//! let sub = Subscription::bind_persisted(&bus, "GTS 1 · Mod A");
//! // Subscriber, audio thread, once per block.
//! assert_eq!(sub.read(), Some(-0.25));
//! // Subscriber, GUI thread, ~4x/s: reconnects if the publisher went away and came back.
//! sub.rebind_if_lost();
//! ```

use std::mem::{offset_of, size_of};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Kernel object name every device opens.
pub const DEFAULT_NAME: &str = "Local\\RiddimRackBus.v1";
pub const MAX_INSTANCES: usize = 64;
pub const MAX_SOURCES: usize = 256;
/// `b"RRB1"` read as a little-endian u32.
pub const MAGIC: u32 = u32::from_le_bytes(*b"RRB1");
pub const VERSION: u32 = 1;
/// Size of the mapping. The layout below fits inside it with room to grow.
pub const MAP_SIZE: usize = 64 * 1024;
/// An instance whose heartbeat is older than this is reclaimed (crashed or force-quit host).
pub const STALE_MS: u64 = 2000;
/// Persisted source strings are `"<label> · <name>"`.
pub const SEPARATOR: &str = " · ";

/// Instance id while its record is being filled in (never a real id: the counter skips u32::MAX).
const CLAIMING: u64 = u64::MAX;
const LABEL_BYTES: usize = 24;
const NAME_BYTES: usize = 24;
const DEVICE_BYTES: usize = 8;

// ---- shared layout (fixed offsets, all atomics) -------------------------------------------

#[repr(C)]
struct Header {
    magic: AtomicU32,
    version: AtomicU32,
    created_ms: AtomicU64,
    /// 0 = fresh (zeroed mapping), 1 = first opener writing the header, 2 = ready.
    state: AtomicU32,
    _pad: AtomicU32,
}

#[repr(C)]
struct InstanceRec {
    /// 0 = free, `CLAIMING` = being set up, else the owner's unique id.
    id: AtomicU64,
    heartbeat_ms: AtomicU64,
    device: [AtomicU64; DEVICE_BYTES / 8],
    label: [AtomicU64; LABEL_BYTES / 8],
}

#[repr(C)]
struct SourceRec {
    /// Owning instance id, 0 = free.
    owner: AtomicU64,
    /// Publisher update counter.
    stamp: AtomicU64,
    /// Bumped by 2 per allocation: odd while the slot is being filled in, even once ready.
    gen: AtomicU32,
    /// f32 bits.
    value: AtomicU32,
    /// [`Kind`] as u32.
    kind: AtomicU32,
    _pad: AtomicU32,
    name: [AtomicU64; NAME_BYTES / 8],
}

#[repr(C)]
struct Shared {
    header: Header,
    instances: [InstanceRec; MAX_INSTANCES],
    sources: [SourceRec; MAX_SOURCES],
}

/// Bytes of the shared layout actually used (header + instance table + source table).
pub const LAYOUT_BYTES: usize = size_of::<Shared>();

const _: () = {
    assert!(offset_of!(Header, magic) == 0);
    assert!(offset_of!(Header, version) == 4);
    assert!(offset_of!(Header, created_ms) == 8);
    assert!(offset_of!(Header, state) == 16);
    assert!(size_of::<Header>() == 24);
    assert!(offset_of!(InstanceRec, heartbeat_ms) == 8);
    assert!(offset_of!(InstanceRec, device) == 16);
    assert!(offset_of!(InstanceRec, label) == 24);
    assert!(size_of::<InstanceRec>() == 48);
    assert!(offset_of!(SourceRec, stamp) == 8);
    assert!(offset_of!(SourceRec, gen) == 16);
    assert!(offset_of!(SourceRec, value) == 20);
    assert!(offset_of!(SourceRec, kind) == 24);
    assert!(offset_of!(SourceRec, name) == 32);
    assert!(size_of::<SourceRec>() == 56);
    assert!(offset_of!(Shared, instances) == 24);
    assert!(offset_of!(Shared, sources) == 24 + MAX_INSTANCES * 48);
    assert!(LAYOUT_BYTES == 17432);
    assert!(LAYOUT_BYTES <= MAP_SIZE);
};

// ---- platform mapping ----------------------------------------------------------------------

struct Mapping {
    ptr: *const Shared,
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

// SAFETY: the mapping only holds atomics and lives until the last `Arc<Mapping>` drops.
unsafe impl Send for Mapping {}
unsafe impl Sync for Mapping {}

#[cfg(windows)]
impl Mapping {
    fn open(name: &str) -> Option<Mapping> {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Memory::{
            CreateFileMappingW, MapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
        };
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: plain Win32 calls; a page-file mapping is zero-filled when first created, and
        // MapViewOfFile fails if an existing mapping of this name is smaller than MAP_SIZE.
        unsafe {
            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null(),
                PAGE_READWRITE,
                0,
                MAP_SIZE as u32,
                wide.as_ptr(),
            );
            if handle.is_null() {
                return None;
            }
            let view = MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, MAP_SIZE);
            if view.Value.is_null() {
                CloseHandle(handle);
                return None;
            }
            Some(Mapping {
                ptr: view.Value as *const Shared,
                handle,
            })
        }
    }
}

#[cfg(windows)]
impl Drop for Mapping {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Memory::{UnmapViewOfFile, MEMORY_MAPPED_VIEW_ADDRESS};
        // SAFETY: ptr/handle came from a successful open and are released exactly once.
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.ptr as *mut _,
            });
            CloseHandle(self.handle);
        }
    }
}

#[cfg(not(windows))]
impl Mapping {
    fn open(name: &str) -> Option<Mapping> {
        use std::sync::Mutex;
        static REGIONS: Mutex<Vec<(String, usize)>> = Mutex::new(Vec::new());
        let mut regions = REGIONS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&(_, p)) = regions.iter().find(|(n, _)| n == name) {
            return Some(Mapping {
                ptr: p as *const Shared,
            });
        }
        // ponytail: one zeroed region per name, leaked for the process lifetime (a named mapping
        // that never closes). Zeroed atomics are valid values.
        let p = unsafe { std::alloc::alloc_zeroed(std::alloc::Layout::new::<Shared>()) };
        if p.is_null() {
            return None;
        }
        regions.push((name.to_owned(), p as usize));
        Some(Mapping {
            ptr: p as *const Shared,
        })
    }
}

/// First opener CASes state 0→1, writes the header, stores 2. Later openers wait briefly for 2
/// and verify magic + version. GUI thread only (may sleep).
fn init_header(s: &Shared) -> bool {
    let h = &s.header;
    if h.state.compare_exchange(0, 1, SeqCst, SeqCst).is_ok() {
        h.magic.store(MAGIC, SeqCst);
        h.version.store(VERSION, SeqCst);
        h.created_ms.store(now_ms(), SeqCst);
        h.state.store(2, SeqCst);
        return true;
    }
    let deadline = Instant::now() + Duration::from_millis(100);
    while h.state.load(SeqCst) != 2 {
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    h.magic.load(SeqCst) == MAGIC && h.version.load(SeqCst) == VERSION
}

// ---- helpers -------------------------------------------------------------------------------

/// Wall-clock milliseconds. On Windows `SystemTime::now` is `GetSystemTimePreciseAsFileTime`,
/// which reads the shared user-mode clock page (no kernel transition, never blocks), so it is
/// safe from the audio thread.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Unique nonzero id: `(pid << 32) | process-local counter` (counter skips 0 and u32::MAX).
fn next_id() -> u64 {
    static COUNTER: AtomicU32 = AtomicU32::new(1);
    loop {
        let c = COUNTER.fetch_add(1, Relaxed);
        if c != 0 && c != u32::MAX {
            return (u64::from(std::process::id()) << 32) | u64::from(c);
        }
    }
}

/// `s` truncated to `cap` bytes on a char boundary (what a fixed field can hold).
fn fit(s: &str, cap: usize) -> &str {
    let mut n = s.len().min(cap);
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    &s[..n]
}

fn write_str(dst: &[AtomicU64], s: &str) {
    let mut buf = [0u8; 24];
    let s = fit(s, dst.len() * 8);
    buf[..s.len()].copy_from_slice(s.as_bytes());
    for (i, w) in dst.iter().enumerate() {
        w.store(
            u64::from_le_bytes(buf[i * 8..i * 8 + 8].try_into().unwrap()),
            SeqCst,
        );
    }
}

fn read_str(src: &[AtomicU64]) -> String {
    let mut buf = [0u8; 24];
    for (i, w) in src.iter().enumerate() {
        buf[i * 8..i * 8 + 8].copy_from_slice(&w.load(SeqCst).to_le_bytes());
    }
    let cap = src.len() * 8;
    let n = buf[..cap].iter().position(|&b| b == 0).unwrap_or(cap);
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

fn live(id: u64) -> bool {
    id != 0 && id != CLAIMING
}

/// Frees instances whose heartbeat is stale and sources whose owner has no live record.
/// GUI thread only (called from register/publish).
fn reclaim(s: &Shared) {
    let now = now_ms();
    for r in &s.instances {
        let id = r.id.load(SeqCst);
        if live(id) && now.saturating_sub(r.heartbeat_ms.load(SeqCst)) > STALE_MS {
            let _ = r.id.compare_exchange(id, 0, SeqCst, SeqCst);
        }
    }
    for src in &s.sources {
        let o = src.owner.load(SeqCst);
        // Records are read after the owner, and an owner's record exists before it claims a
        // source, so a live owner is always found here.
        if o != 0 && !s.instances.iter().any(|r| r.id.load(SeqCst) == o) {
            let _ = src.owner.compare_exchange(o, 0, SeqCst, SeqCst);
        }
    }
}

// ---- public API ----------------------------------------------------------------------------

/// Signal range, for display and for subscribers that want to rescale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// 0..1
    Unipolar = 0,
    /// -1..1
    Bipolar = 1,
    /// 0 or 1
    Gate = 2,
}

impl Kind {
    fn from_u32(v: u32) -> Kind {
        match v {
            1 => Kind::Bipolar,
            2 => Kind::Gate,
            _ => Kind::Unipolar,
        }
    }
}

/// One live source as seen by [`Bus::list`].
#[derive(Clone, Debug, PartialEq)]
pub struct SourceInfo {
    pub label: String,
    pub device: String,
    pub name: String,
    pub kind: Kind,
    pub owner: u64,
    pub slot: usize,
    pub gen: u32,
}

impl SourceInfo {
    /// `"<label> · <name>"`, the string devices persist and show in source combos.
    pub fn persisted(&self) -> String {
        format!("{}{SEPARATOR}{}", self.label, self.name)
    }
}

/// Handle to the shared bus. Cheap to clone; `Send + Sync`.
#[derive(Clone)]
pub struct Bus(Option<Arc<Mapping>>);

impl Bus {
    /// Opens (or creates) the session bus. Never fails; may return a disabled bus.
    pub fn open() -> Bus {
        Bus::open_named(DEFAULT_NAME)
    }

    /// Opens a bus under a specific kernel object name (tests use a unique name per process).
    pub fn open_named(name: &str) -> Bus {
        match Mapping::open(name) {
            // SAFETY: the mapping is MAP_SIZE >= LAYOUT_BYTES, page-aligned, and all atomics.
            Some(m) if init_header(unsafe { &*m.ptr }) => Bus(Some(Arc::new(m))),
            _ => Bus(None),
        }
    }

    /// A bus on which every operation is a no-op.
    pub fn disabled() -> Bus {
        Bus(None)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.is_some()
    }

    fn shared(&self) -> Option<&Shared> {
        // SAFETY: see open_named; the Arc keeps the view mapped while `self` lives.
        self.0.as_ref().map(|m| unsafe { &*m.ptr })
    }

    /// Claims an instance record (GUI thread). Label defaults to `"<device> N"`, lowest free N.
    /// Returns a disabled instance if the bus is disabled or all 64 records are taken.
    pub fn register(&self, device: &str) -> Instance {
        let Some(s) = self.shared() else {
            return Instance::disabled();
        };
        reclaim(s);
        let id = next_id();
        for (idx, r) in s.instances.iter().enumerate() {
            if r.id.compare_exchange(0, CLAIMING, SeqCst, SeqCst).is_ok() {
                // Heartbeat is fresh before the id goes live, so a concurrent reclaim never
                // sees a live id with a stale heartbeat.
                r.heartbeat_ms.store(now_ms(), SeqCst);
                let device = fit(device, DEVICE_BYTES);
                write_str(&r.device, device);
                let taken: Vec<String> = s
                    .instances
                    .iter()
                    .filter(|o| live(o.id.load(SeqCst)))
                    .map(|o| read_str(&o.label))
                    .collect();
                let label = (1..)
                    .map(|n| format!("{device} {n}"))
                    .find(|l| !taken.contains(l))
                    .unwrap();
                write_str(&r.label, &label);
                r.id.store(id, SeqCst);
                return Instance {
                    bus: self.clone(),
                    idx,
                    id,
                };
            }
        }
        Instance::disabled()
    }

    /// Every live, fully published source (GUI thread; allocates).
    pub fn list(&self) -> Vec<SourceInfo> {
        let Some(s) = self.shared() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (slot, src) in s.sources.iter().enumerate() {
            let gen = src.gen.load(SeqCst);
            let owner = src.owner.load(SeqCst);
            if gen % 2 == 1 || owner == 0 {
                continue;
            }
            let Some(rec) = s.instances.iter().find(|r| r.id.load(SeqCst) == owner) else {
                continue;
            };
            let info = SourceInfo {
                label: read_str(&rec.label),
                device: read_str(&rec.device),
                name: read_str(&src.name),
                kind: Kind::from_u32(src.kind.load(SeqCst)),
                owner,
                slot,
                gen,
            };
            if src.gen.load(SeqCst) == gen && src.owner.load(SeqCst) == owner {
                out.push(info);
            }
        }
        out
    }

    /// (owner, slot, gen) of the first live source matching label + name.
    fn resolve(&self, label: &str, name: &str) -> Option<(u64, usize, u32)> {
        let s = self.shared()?;
        let (label, name) = (fit(label, LABEL_BYTES), fit(name, NAME_BYTES));
        for rec in &s.instances {
            let id = rec.id.load(SeqCst);
            if !live(id) || read_str(&rec.label) != label {
                continue;
            }
            for (slot, src) in s.sources.iter().enumerate() {
                let gen = src.gen.load(SeqCst);
                if gen % 2 == 0
                    && src.owner.load(SeqCst) == id
                    && read_str(&src.name) == name
                    && src.gen.load(SeqCst) == gen
                {
                    return Some((id, slot, gen));
                }
            }
        }
        None
    }
}

/// A registered device instance. Dropping it frees its record and all its sources.
pub struct Instance {
    bus: Bus,
    idx: usize,
    /// 0 = disabled.
    id: u64,
}

impl Instance {
    fn disabled() -> Instance {
        Instance {
            bus: Bus::disabled(),
            idx: 0,
            id: 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn is_enabled(&self) -> bool {
        self.owned().is_some()
    }

    /// The record, if this instance still owns it (it may have been reclaimed as stale).
    fn owned(&self) -> Option<&InstanceRec> {
        if self.id == 0 {
            return None;
        }
        let r = &self.bus.shared()?.instances[self.idx];
        (r.id.load(Acquire) == self.id).then_some(r)
    }

    /// Audio thread, once per block: two atomic ops and one wall-clock read. May also be called
    /// from the GUI thread when the host is not processing, to keep the record from going stale.
    pub fn heartbeat(&self) {
        if let Some(r) = self.owned() {
            r.heartbeat_ms.store(now_ms(), Release);
        }
    }

    /// GUI thread. Truncated to 24 bytes.
    pub fn set_label(&self, label: &str) {
        if let Some(r) = self.owned() {
            write_str(&r.label, label);
        }
    }

    /// GUI thread. Empty if disabled.
    pub fn label(&self) -> String {
        self.owned().map(|r| read_str(&r.label)).unwrap_or_default()
    }

    /// Claims a source slot (GUI thread). Name truncated to 24 bytes. Returns a disabled source
    /// if this instance is disabled/reclaimed or all 256 slots are taken.
    pub fn publish(&self, name: &str, kind: Kind) -> Source {
        let disabled = Source {
            bus: Bus::disabled(),
            idx: 0,
            owner: 0,
            gen: 0,
        };
        let Some(r) = self.owned() else {
            return disabled;
        };
        r.heartbeat_ms.store(now_ms(), SeqCst); // never reclaim ourselves below
        let s = self.bus.shared().unwrap();
        reclaim(s);
        if self.owned().is_none() {
            return disabled;
        }
        for (idx, src) in s.sources.iter().enumerate() {
            if src
                .owner
                .compare_exchange(0, self.id, SeqCst, SeqCst)
                .is_ok()
            {
                src.gen.fetch_add(1, SeqCst); // odd: filling in
                write_str(&src.name, name);
                src.kind.store(kind as u32, SeqCst);
                src.value.store(0f32.to_bits(), SeqCst);
                src.stamp.store(0, SeqCst);
                let gen = src.gen.fetch_add(1, SeqCst).wrapping_add(1); // even: ready
                return Source {
                    bus: self.bus.clone(),
                    idx,
                    owner: self.id,
                    gen,
                };
            }
        }
        disabled
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        if self.id == 0 {
            return;
        }
        if let Some(s) = self.bus.shared() {
            for src in &s.sources {
                let _ = src.owner.compare_exchange(self.id, 0, SeqCst, SeqCst);
            }
            let _ = s.instances[self.idx]
                .id
                .compare_exchange(self.id, 0, SeqCst, SeqCst);
        }
    }
}

/// A published signal. Dropping it frees the slot.
pub struct Source {
    bus: Bus,
    idx: usize,
    /// 0 = disabled.
    owner: u64,
    gen: u32,
}

impl Source {
    pub fn is_enabled(&self) -> bool {
        self.slot().is_some()
    }

    /// The slot, if this source still owns it.
    fn slot(&self) -> Option<&SourceRec> {
        if self.owner == 0 {
            return None;
        }
        let src = &self.bus.shared()?.sources[self.idx];
        (src.owner.load(Acquire) == self.owner && src.gen.load(Acquire) == self.gen).then_some(src)
    }

    /// Audio thread, once per block. No-op if disabled or the slot was reclaimed.
    pub fn set(&self, value: f32) {
        if let Some(src) = self.slot() {
            src.value.store(value.to_bits(), Release);
            src.stamp.fetch_add(1, Release);
        }
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        if let Some(src) = self.slot() {
            let _ = src.owner.compare_exchange(self.owner, 0, SeqCst, SeqCst);
        }
    }
}

/// A CV-in binding to a source identified by (label, name). `read` is for the audio thread,
/// `rebind_if_lost` for the GUI thread; share it between them (e.g. in an `Arc`).
pub struct Subscription {
    bus: Bus,
    label: String,
    name: String,
    /// Bound owner id, 0 = unbound. Written last on rebind (Release) so `slot_gen` is valid.
    owner: AtomicU64,
    /// `(slot << 32) | gen`.
    slot_gen: AtomicU64,
    lost: AtomicBool,
}

impl Subscription {
    /// Binds to the first live source with this label + name, or stays lost until it appears
    /// (GUI thread).
    pub fn bind(bus: &Bus, label: &str, name: &str) -> Subscription {
        let sub = Subscription {
            bus: bus.clone(),
            label: label.to_owned(),
            name: name.to_owned(),
            owner: AtomicU64::new(0),
            slot_gen: AtomicU64::new(0),
            lost: AtomicBool::new(true),
        };
        sub.rebind_if_lost();
        sub
    }

    /// Binds from a persisted `"<label> · <name>"` string (GUI thread). A string without the
    /// separator is treated as a bare name with an empty label (never matches).
    pub fn bind_persisted(bus: &Bus, persisted: &str) -> Subscription {
        let (label, name) = persisted.rsplit_once(SEPARATOR).unwrap_or(("", persisted));
        Subscription::bind(bus, label, name)
    }

    /// `"<label> · <name>"` for saving in device state.
    pub fn persisted(&self) -> String {
        format!("{}{SEPARATOR}{}", self.label, self.name)
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// True when unbound or the bound source went away (GUI shows the name dimmed).
    pub fn is_lost(&self) -> bool {
        self.lost.load(Relaxed)
    }

    /// Audio thread, once per block: atomic loads only. `None` when unbound or the source was
    /// freed/reused, in which case the subscription is flagged lost.
    pub fn read(&self) -> Option<f32> {
        let owner = self.owner.load(Acquire);
        if owner == 0 {
            return None;
        }
        let sg = self.slot_gen.load(Acquire);
        let (slot, gen) = ((sg >> 32) as usize, sg as u32);
        let src = &self.bus.shared()?.sources[slot];
        let valid =
            |src: &SourceRec| src.owner.load(Acquire) == owner && src.gen.load(Acquire) == gen;
        if valid(src) {
            let bits = src.value.load(Acquire);
            if valid(src) {
                return Some(f32::from_bits(bits));
            }
        }
        self.lost.store(true, Relaxed);
        None
    }

    /// GUI thread: if lost, re-resolve by (label, name). One atomic load when still bound.
    /// Returns whether the subscription is bound afterwards.
    pub fn rebind_if_lost(&self) -> bool {
        if !self.lost.load(Relaxed) {
            return true;
        }
        let Some((owner, slot, gen)) = self.bus.resolve(&self.label, &self.name) else {
            return false;
        };
        self.owner.store(0, Release);
        self.slot_gen
            .store(((slot as u64) << 32) | u64::from(gen), Release);
        self.owner.store(owner, Release);
        self.lost.store(false, Relaxed);
        true
    }
}

#[cfg(test)]
mod tests;
