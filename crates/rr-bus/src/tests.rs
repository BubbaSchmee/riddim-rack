//! SPEC Verification "rr-bus unit tests". Each test opens its own uniquely named bus so tests
//! never share state with each other or with a running DAW.

use super::*;
use std::sync::atomic::AtomicUsize;

fn unique_name(tag: &str) -> String {
    static N: AtomicUsize = AtomicUsize::new(0);
    format!(
        "Local\\RiddimRackBusTest.{tag}.{}.{}",
        std::process::id(),
        N.fetch_add(1, Relaxed)
    )
}

fn unique_bus() -> Bus {
    let bus = Bus::open_named(&unique_name("t"));
    assert!(bus.is_enabled());
    bus
}

#[test]
fn send_sync_and_layout_size() {
    fn ss<T: Send + Sync>() {}
    ss::<Bus>();
    ss::<Instance>();
    ss::<Source>();
    ss::<Subscription>();
    assert_eq!(LAYOUT_BYTES, 17432);
}

#[test]
fn two_opens_share_state() {
    let name = unique_name("share");
    let a = Bus::open_named(&name);
    let b = Bus::open_named(&name);
    assert!(a.is_enabled() && b.is_enabled());
    let inst = a.register("GTS");
    let src = inst.publish("Mod A", Kind::Bipolar);
    src.set(0.5);
    let listed = b.list();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].persisted(), "GTS 1 · Mod A");
    assert_eq!(Subscription::bind(&b, "GTS 1", "Mod A").read(), Some(0.5));
}

#[test]
fn publish_read_round_trip() {
    let bus = unique_bus();
    let g1 = bus.register("GTS");
    let g2 = bus.register("GTS");
    assert_eq!(
        (g1.label().as_str(), g2.label().as_str()),
        ("GTS 1", "GTS 2")
    );
    let env = g2.publish("Filter Env", Kind::Unipolar);
    let gate = g2.publish("Gate", Kind::Gate);
    let sub = Subscription::bind_persisted(&bus, "GTS 2 · Filter Env");
    assert_eq!(sub.persisted(), "GTS 2 · Filter Env");
    assert_eq!(sub.read(), Some(0.0));
    for v in [0.25f32, 1.0, -0.75, 0.0] {
        env.set(v);
        g2.heartbeat();
        assert_eq!(sub.read(), Some(v));
    }
    let kinds: Vec<_> = bus.list().into_iter().map(|i| (i.name, i.kind)).collect();
    assert_eq!(
        kinds,
        [
            ("Filter Env".into(), Kind::Unipolar),
            ("Gate".into(), Kind::Gate)
        ]
    );
    // Relabel keeps the live binding; new binds use the new label.
    g2.set_label("Lead");
    assert_eq!(sub.read(), Some(0.0));
    gate.set(1.0);
    assert_eq!(Subscription::bind(&bus, "Lead", "Gate").read(), Some(1.0));
    assert!(Subscription::bind(&bus, "GTS 2", "Nope").read().is_none());
}

#[test]
fn gen_bump_prevents_stale_binding() {
    let bus = unique_bus();
    let inst = bus.register("VSS");
    let first = inst.publish("Vowel", Kind::Unipolar);
    first.set(0.25);
    let sub = Subscription::bind(&bus, "VSS 1", "Vowel");
    assert_eq!(sub.read(), Some(0.25));
    drop(first);
    // Same slot reused by a different source of another instance.
    let other = bus.register("GTS");
    let intruder = other.publish("Mod A", Kind::Bipolar);
    intruder.set(-1.0);
    assert_eq!(bus.list()[0].slot, 0);
    assert_eq!(sub.read(), None);
    assert!(sub.is_lost());
    assert!(!sub.rebind_if_lost());
    drop(intruder);
    // Same slot reused by the same owner and name: gen differs, so still lost until rebind.
    let again = inst.publish("Vowel", Kind::Unipolar);
    again.set(0.75);
    assert_eq!(sub.read(), None);
    assert!(sub.rebind_if_lost());
    assert_eq!(sub.read(), Some(0.75));
}

#[test]
fn stale_heartbeat_reclaim() {
    let bus = unique_bus();
    let crashed = bus.register("GTS");
    let src = crashed.publish("Mod A", Kind::Bipolar);
    src.set(0.5);
    let sub = Subscription::bind(&bus, "GTS 1", "Mod A");
    assert_eq!(sub.read(), Some(0.5));
    let s = bus.shared().unwrap();
    s.instances[crashed.idx]
        .heartbeat_ms
        .store(now_ms() - STALE_MS - 1000, SeqCst);
    let fresh = bus.register("VSS");
    assert_eq!(s.instances[crashed.idx].id.load(SeqCst), fresh.id()); // record reused
    assert!(!crashed.is_enabled() && !src.is_enabled());
    assert_eq!(sub.read(), None);
    assert!(bus.list().is_empty());
    // The zombie's late calls and drops must not touch the new owner.
    crashed.heartbeat();
    src.set(1.0);
    drop(src);
    drop(crashed);
    assert!(fresh.is_enabled());
    assert_eq!(fresh.label(), "VSS 1");
}

#[test]
fn full_tables_allocate_and_free_without_leaks() {
    let bus = unique_bus();
    for _round in 0..2 {
        let mut held = Vec::new();
        for i in 0..MAX_INSTANCES {
            let inst = bus.register("GTS");
            assert!(inst.is_enabled(), "instance {i}");
            let srcs: Vec<_> = (0..4)
                .map(|k| inst.publish(&format!("S{k}"), Kind::Unipolar))
                .collect();
            assert!(srcs.iter().all(Source::is_enabled));
            held.push((inst, srcs));
        }
        assert!(!bus.register("GTS").is_enabled());
        assert!(!held[0].0.publish("extra", Kind::Gate).is_enabled());
        assert_eq!(bus.list().len(), MAX_SOURCES);
        // Drop half the sources before their instances, half after.
        for (i, (inst, srcs)) in held.into_iter().enumerate() {
            if i % 2 == 0 {
                drop(srcs);
                drop(inst);
            } else {
                drop(inst);
                drop(srcs);
            }
        }
        let s = bus.shared().unwrap();
        assert!(s.instances.iter().all(|r| r.id.load(SeqCst) == 0));
        assert!(s.sources.iter().all(|r| r.owner.load(SeqCst) == 0));
        assert!(bus.list().is_empty());
    }
}

#[test]
fn corrupt_header_disables_without_panic() {
    let name = unique_name("corrupt");
    let good = Bus::open_named(&name);
    let h = &good.shared().unwrap().header;
    h.magic.store(0xDEAD_BEEF, SeqCst);
    let bad = Bus::open_named(&name);
    assert!(!bad.is_enabled());
    h.magic.store(MAGIC, SeqCst);
    h.version.store(VERSION + 1, SeqCst);
    assert!(!Bus::open_named(&name).is_enabled());
    h.version.store(VERSION, SeqCst);
    h.state.store(1, SeqCst); // creator died mid-init
    assert!(!Bus::open_named(&name).is_enabled());
    // Everything on a disabled bus is a quiet no-op.
    let inst = bad.register("GTS");
    let src = inst.publish("Mod A", Kind::Bipolar);
    src.set(1.0);
    inst.heartbeat();
    inst.set_label("x");
    let sub = Subscription::bind(&bad, "GTS 1", "Mod A");
    assert_eq!(sub.read(), None);
    assert!(!sub.rebind_if_lost());
    assert!(bad.list().is_empty());
    assert!(!Bus::disabled().register("GTS").is_enabled());
}

#[test]
fn four_thread_stress_has_no_torn_values() {
    // Values whose high and low 16 bits match; a torn write would break the pattern.
    // i < 0x7F00 keeps them finite, positive floats.
    fn pattern(i: u32) -> f32 {
        let i = i % 0x7F00;
        f32::from_bits((i << 16) | i)
    }
    fn intact(v: f32) -> bool {
        let b = v.to_bits();
        b >> 16 == b & 0xFFFF
    }
    let bus = unique_bus();
    let stop = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();
    for p in 0..2 {
        let (bus, stop) = (bus.clone(), stop.clone());
        handles.push(std::thread::spawn(move || {
            let inst = bus.register("PUB");
            let mut src = inst.publish("Sig", Kind::Unipolar);
            let mut i = 0u32;
            while !stop.load(Relaxed) {
                src.set(pattern(i));
                inst.heartbeat();
                i = i.wrapping_add(1);
                // One publisher also churns its slot so readers exercise gen validation.
                if p == 1 && i.is_multiple_of(5000) {
                    drop(src);
                    src = inst.publish("Sig", Kind::Unipolar);
                }
            }
            0usize
        }));
    }
    while bus.list().len() < 2 {
        std::thread::yield_now();
    }
    for _ in 0..2 {
        let (bus, stop) = (bus.clone(), stop.clone());
        handles.push(std::thread::spawn(move || {
            let subs: Vec<_> = ["PUB 1", "PUB 2"]
                .iter()
                .map(|l| Subscription::bind(&bus, l, "Sig"))
                .collect();
            let mut reads = 0usize;
            while !stop.load(Relaxed) {
                for sub in &subs {
                    match sub.read() {
                        Some(v) => {
                            assert!(intact(v), "torn value {:#x}", v.to_bits());
                            reads += 1;
                        }
                        None => {
                            sub.rebind_if_lost();
                        }
                    }
                }
            }
            reads
        }));
    }
    std::thread::sleep(Duration::from_secs(1));
    stop.store(true, Relaxed);
    let reads: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    assert!(reads > 1000, "only {reads} reads");
}

#[test]
fn labels_stay_unique_and_renames_are_visible_to_subscribers() {
    let bus = unique_bus();
    let g1 = bus.register("GTS");
    let g2 = bus.register("GTS");
    // A duplicated track / preset load asking for a label another instance holds.
    assert_eq!(g2.set_label("GTS 1"), "GTS 1 (2)");
    assert_eq!(g2.label(), "GTS 1 (2)");
    assert_eq!(g1.set_label("GTS 1"), "GTS 1"); // own label is not a clash
    let long = "A label that is far too long";
    assert_eq!(g1.set_label(long), fit(long, LABEL_BYTES));
    let v = g2.set_label(long);
    assert!(v.ends_with(" (2)") && v.len() <= LABEL_BYTES, "{v}");
    // The bound owner's current label, for re-persisting after a rename.
    let src = g1.publish("Mod A", Kind::Bipolar);
    src.set(0.5);
    let sub = Subscription::bind(&bus, &g1.label(), "Mod A");
    g1.set_label("Bass");
    assert_eq!(sub.live_persisted().as_deref(), Some("Bass · Mod A"));
    assert!(Subscription::bind(&bus, "nobody", "Mod A").live_persisted().is_none());
}

#[test]
fn idle_publisher_reads_none_until_it_resumes() {
    let bus = unique_bus();
    let inst = bus.register("GTS");
    let gate = inst.publish("Gate", Kind::Gate);
    gate.set(1.0);
    let sub = Subscription::bind(&bus, "GTS 1", "Gate");
    assert_eq!(sub.read(), Some(1.0));
    std::thread::sleep(Duration::from_millis(IDLE_MS + 60));
    assert_eq!(sub.read(), None, "a publisher that stopped processing must not hold its gate");
    assert!(!sub.is_lost());
    gate.set(1.0);
    assert_eq!(sub.read(), Some(1.0));
}

#[test]
fn reclaimed_but_unused_record_is_taken_back_by_heartbeat() {
    let bus = unique_bus();
    let paused = bus.register("GTS");
    let src = paused.publish("Mod A", Kind::Bipolar);
    src.set(0.5);
    let sub = Subscription::bind(&bus, "GTS 1", "Mod A");
    let s = bus.shared().unwrap();
    s.instances[paused.idx].heartbeat_ms.store(now_ms() - STALE_MS - 1000, SeqCst);
    reclaim(s);
    assert!(!paused.is_enabled() && !src.is_enabled());
    assert_eq!(sub.read(), None);
    // The host resumes process(): heartbeat + set take the record and slot back.
    paused.heartbeat();
    src.set(0.75);
    assert!(paused.is_enabled() && src.is_enabled());
    assert_eq!(sub.read(), Some(0.75));
    assert_eq!(bus.list().len(), 1);
}

#[test]
fn resurrected_record_is_deduplicated_against_a_label_taken_meanwhile() {
    let bus = unique_bus();
    let a = bus.register("GTS");
    assert_eq!(a.set_label("Lead"), "Lead");
    let b = bus.register("VSS");
    let s = bus.shared().unwrap();
    s.instances[a.idx].heartbeat_ms.store(now_ms() - STALE_MS - 1000, SeqCst);
    s.instances[b.idx].heartbeat_ms.store(now_ms(), SeqCst);
    reclaim(s); // A's record is free and unused
    assert_eq!(b.set_label("Lead"), "Lead");
    a.heartbeat(); // resurrects with the old label
    let l = a.label();
    assert_eq!(a.set_label(&l), "Lead (2)");
    assert_eq!(b.label(), "Lead");
}
