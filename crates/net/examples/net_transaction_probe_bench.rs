mod bench {
//! Probe (not for landing): bytes per snapshot section and the cost of the
//! host encode and the client decode path, on a synthetic snapshot.
//!
//! The snapshot is built from public `sim` types: `players` player states that
//! move every tick, `entities` entity states and as many occupied entity-kernel
//! slots, `glass` glass pieces of which `churn` change per tick. No game data.
//! The client side repeats the steps of the snapshot arm of
//! `UdpSession::handle_server_packet` one by one and times each of them.
//!
//! ```text
//! cargo run -p net --example net_frame_bench --profile play -- [players] [entities] [glass] [churn] [ticks] [lag]
//! ```
//! `lag`: how many snapshots behind the newest one the baseline is (the host
//! encodes against the last acked snapshot).

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Instant;

use net::transport::wire::WireReader;
use net::{Frame, SnapshotDecoder, SnapshotEncoder, WorldObjectSyncDecoder, frame_from_acked_tick};
use playerstate_iw4::PlayerState;
use sim::{
    ClientId, ClientSnapshotMeta, EntityKernelOccupiedSnapshot, EntityKernelSlotSnapshot,
    EntityRelations, EntityRunKind, GlassCause, GlassPieceSnapshot, GlassPieceState, Snapshot,
    Tick, TickInput,
};

pub fn snapshot(tick: u32, players: usize, entities: usize, glass: usize, churn: usize) -> Snapshot {
    let t = tick as f32;
    let mut snap = Snapshot::unpublished(Tick(tick));
    for i in 0..players {
        let mut ps = PlayerState::ZERO;
        let f = i as f32;
        ps.origin = [f * 64.0 + t * 3.0, f * 32.0 - t * 2.0, 16.0];
        ps.velocity = [190.0, -120.0, 0.0];
        ps.viewangles = [(t * 0.7 + f) % 80.0 - 40.0, (t * 1.3 + f * 10.0) % 360.0, 0.0];
        ps.weapon = 1 + (i as u32 % 5);
        snap.players.push((ClientId(i as u32), ps));
        let mut meta = ClientSnapshotMeta::default();
        meta.ammo_clip = 30 - (tick as i32 % 30);
        snap.meta.clients.push((ClientId(i as u32), meta));
    }
    let base = playerstate_iw4::GENTITY_SPAWN_BASE;
    for e in 0..entities {
        let mut es = entity_iw4::EntityState::default();
        es.number = base + e as i32;
        es.e_type = 1 + (e as i32 % 6);
        es.tr_base = [e as f32 * 8.0, (e % 7) as f32 * 100.0, 0.0];
        if e % 10 == 0 {
            es.tr_base[2] = (t * 0.5).sin() * 20.0;
        }
        snap.meta.entities.push(es);
    }
    let kernel = &mut snap.meta.entity_kernel;
    kernel.high_water = base + entities as i32;
    kernel.level_time_ms = tick as i32 * 50;
    kernel.frame_serial = tick;
    kernel.slots = (0..entities)
        .map(|e| EntityKernelSlotSnapshot {
            generation: 1,
            occupied: Some(EntityKernelOccupiedSnapshot {
                kind: if e % 3 == 0 {
                    EntityRunKind::ScriptMover
                } else {
                    EntityRunKind::General
                },
                linked: true,
                relations: EntityRelations::default(),
                next_think_ms: (e % 4 == 0).then_some(tick as i32 * 50 + 50),
                transient_event_time_ms: None,
            }),
            freed_at_ms: 0,
        })
        .collect();
    let world = &mut snap.meta.world_objects;
    world.as_of_ms = tick as i32 * 50;
    world.glass_pieces = (0..glass as u32)
        .map(|id| {
            let hit = churn > 0 && (id as usize) < (tick as usize * churn).min(glass);
            (
                id * 3 + 7,
                GlassPieceSnapshot {
                    state: if hit {
                        GlassPieceState::Weakened
                    } else {
                        GlassPieceState::Intact
                    },
                    revision: u32::from(hit),
                    last_state_change_time: 0,
                    shatter_seed: None,
                    deterministic_seed: u64::from(id) * 0x9e37_79b9,
                    cause: GlassCause::Impact,
                },
            )
        })
        .collect();
    snap
}

struct Stats(BTreeMap<&'static str, Vec<f64>>);

impl Stats {
    fn time<R>(&mut self, name: &'static str, f: impl FnOnce() -> R) -> R {
        let started = Instant::now();
        let r = f();
        self.0
            .entry(name)
            .or_default()
            .push(started.elapsed().as_secs_f64() * 1e6);
        r
    }

    fn print(&self) {
        println!("{:<34} {:>10} {:>10} {:>10}", "step (µs)", "mean", "p50", "p95");
        for (name, samples) in &self.0 {
            let mut s = samples.clone();
            s.sort_by(f64::total_cmp);
            let mean = s.iter().sum::<f64>() / s.len() as f64;
            let at = |p: f64| s[((s.len() - 1) as f64 * p) as usize];
            println!("{name:<34} {mean:>10.2} {:>10.2} {:>10.2}", at(0.5), at(0.95));
        }
    }
}

pub fn run() {
    let args: Vec<usize> = std::env::args()
        .skip(2)
        .map(|a| a.parse().expect("arguments are integers"))
        .collect();
    let arg = |i: usize, d: usize| args.get(i).copied().unwrap_or(d);
    let (players, entities, glass, churn, ticks, lag) =
        (arg(0, 18), arg(1, 400), arg(2, 300), arg(3, 1), arg(4, 2000), arg(5, 3).max(1));

    // The probe mirrors the snapshot arm of the exact checkout being built.
    let udp_source = include_str!("../src/transport/udp_session.rs");
    let owns_baseline = udp_source.contains("self.baselines.get(&baseline_seq).cloned()");
    let borrows_baseline = udp_source.contains("let Some(baseline) = self.baselines.get(&baseline_seq) else {");
    assert_ne!(owns_baseline, borrows_baseline, "unrecognized snapshot baseline path");
    println!("probe baseline path: {}", if owns_baseline { "clone" } else { "borrow" });
    let mut stats = Stats(BTreeMap::new());
    let mut host_baselines: BTreeMap<u32, Snapshot> = BTreeMap::new();
    let mut client_baselines: BTreeMap<u32, Snapshot> = BTreeMap::new();
    let mut encoder = SnapshotEncoder::new();
    let input = TickInput::default();
    let mut sections = None;
    let mut section_samples: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut bytes_total = 0usize;
    for tick in 1..=ticks as u32 {
        let snap = snapshot(tick, players, entities, glass, churn);
        // Host: one peer, encoded against the snapshot `lag` ticks back.
        let baseline_seq = tick.saturating_sub(lag as u32);
        let payload = stats.time("host encode (adopt+frame+bytes)", || {
            let mut enc = std::mem::take(&mut encoder);
            match host_baselines.get(&baseline_seq) {
                Some(baseline) => enc.adopt_baseline(baseline),
                None => enc.reset(),
            }
            let frame = frame_from_acked_tick(&mut enc, &input, &snap, Vec::new());
            let bytes = frame.to_bytes();
            encoder = enc;
            (frame, bytes)
        });
        let (frame_out, bytes) = payload;
        let section = frame_out.section_bytes();
        let meta = &section.meta;
        for (name, value) in [
            ("frame total", section.total), ("header", section.header),
            ("snapshot delta", section.snapshot_delta), ("reliable", section.reliable),
            ("svc", section.svc), ("meta total", meta.total()),
            ("match_header", meta.match_header), ("events", meta.events),
            ("aliases", meta.aliases), ("entity_dobjs", meta.entity_dobjs),
            ("corpses", meta.corpses), ("entities", meta.entities),
            ("script_movers", meta.script_movers), ("entity_kernel", meta.entity_kernel),
            ("item_tables", meta.item_tables), ("area_entities", meta.area_entities),
            ("objectives", meta.objectives), ("world_objects", meta.world_objects),
        ] {
            section_samples.entry(name).or_default().push(value);
        }
        if tick == ticks as u32 {
            sections = Some(section);
        }
        bytes_total += bytes.len();
        let baseline_seq = if host_baselines.contains_key(&baseline_seq) {
            baseline_seq
        } else {
            0
        };
        host_baselines.insert(tick, snap);
        while host_baselines.len() > 64 {
            let first = *host_baselines.keys().next().unwrap();
            host_baselines.remove(&first);
        }

        // Client: the snapshot arm of the UDP session, step by step.
        let client_started = Instant::now();
        let baseline = stats.time("client baseline lookup", || {
            if baseline_seq == 0 {
                None
            } else if owns_baseline {
                client_baselines.get(&baseline_seq).cloned().map(Cow::Owned)
            } else {
                client_baselines.get(&baseline_seq).map(Cow::Borrowed)
            }
        });
        let mut decoder = SnapshotDecoder::new();
        let mut world_decoder = WorldObjectSyncDecoder::default();
        if let Some(baseline) = baseline.as_ref() {
            stats.time("client decoder adopt", || decoder.adopt_baseline(baseline));
            stats.time("client world adopt", || {
                world_decoder.adopt_baseline(baseline.meta.world_objects.clone())
            });
        }
        let frame = stats.time("client Frame::decode", || {
            let mut input = WireReader::new(&bytes);
            Frame::decode(&mut input, &mut world_decoder).expect("frame decodes")
        });
        let mut snapshot = stats.time("client delta decode", || {
            decoder.decode(&frame.snapshot_delta).expect("delta decodes")
        });
        stats.time("client meta clone", || {
            snapshot.meta = frame.snapshot_meta.clone()
        });
        drop(baseline);
        let kept = stats.time("client snapshot clone", || snapshot.clone());
        client_baselines.insert(tick, kept);
        while client_baselines.len() > 64 {
            let first = *client_baselines.keys().next().unwrap();
            client_baselines.remove(&first);
        }
        stats.0.entry("client total mirrored packet").or_default()
            .push(client_started.elapsed().as_secs_f64() * 1e6);
        assert_eq!(snapshot.players.len(), players);
    }

    println!(
        "players={players} entities={entities} glass={glass} churn={churn} ticks={ticks} lag={lag}"
    );
    let s = sections.unwrap();
    println!(
        "last frame bytes: total {} | header {} delta {} reliable {} svc {}",
        s.total, s.header, s.snapshot_delta, s.reliable, s.svc
    );
    let m = s.meta;
    println!(
        "  meta {}: match_header {} events {} aliases {} entity_dobjs {} corpses {} entities {} script_movers {} entity_kernel {} item_tables {} area_entities {} objectives {} world_objects {}",
        m.total(),
        m.match_header,
        m.events,
        m.aliases,
        m.entity_dobjs,
        m.corpses,
        m.entities,
        m.script_movers,
        m.entity_kernel,
        m.item_tables,
        m.area_entities,
        m.objectives,
        m.world_objects
    );
    println!("mean frame bytes {:.0}", bytes_total as f64 / ticks as f64);
    stats.print();
    println!("section bytes across all ticks: mean median min max n");
    for (name, values) in &mut section_samples {
        values.sort_unstable();
        let mean = values.iter().sum::<usize>() as f64 / values.len() as f64;
        println!("section {name}: {:.2} {} {} {} {}", mean, values[values.len() / 2],
                 values[0], values[values.len() - 1], values.len());
    }

    // Glass delta on the client alone: a table of `glass` pieces, `glass/2`
    // changed and `glass/2` removed in one delta.
    for size in [glass, 4096, 16384] {
        let mut enc = net::WorldObjectSyncEncoder::default();
        let mut full = snapshot(1, 0, 0, size, 0).meta.world_objects;
        let wire_full = net::encode_world_object_sync(&mut enc, Tick(200), &full);
        for (i, (_, row)) in full.glass_pieces.iter_mut().enumerate() {
            if i % 2 == 0 {
                row.revision += 1;
            }
        }
        full.glass_pieces.retain(|(id, _)| id % 4 != 3);
        let wire_delta = net::encode_world_object_sync(&mut enc, Tick(201), &full);
        let mut samples = Vec::new();
        for _ in 0..5 {
            let mut dec = WorldObjectSyncDecoder::default();
            net::decode_world_object_sync_wire(&mut dec, &wire_full).unwrap();
            let started = Instant::now();
            let state = net::decode_world_object_sync_wire(&mut dec, &wire_delta).unwrap();
            samples.push(started.elapsed().as_secs_f64() * 1e3);
            assert_eq!(state.glass_pieces, full.glass_pieces);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "glass delta apply: table {size}, delta {} bytes: median {:.3} ms",
            wire_delta.len(),
            samples[2]
        );
    }
}

}

use net::{Frame, SnapshotDelta, SnapshotDecoder, SnapshotEncoder, WorldObjectSyncDecoder, WorldObjectSyncEncoder, frame_from_acked_tick};
use net::transport::wire::{WireReader, WireWriter};
use sim::{Tick, TickInput};
fn run_probe() {
    let mut baseline = bench::snapshot(1, 3, 6, 7, 1);
    baseline.projectiles.push(sim::ProjectileState {
        id:sim::ProjectileId(1), owner:sim::ClientId(1), owner_life:sim::LifeSequence(0), weapon:1,
        origin:[0.0;3], velocity:[0.0;3], pos:entity_iw4::Trajectory::default(),
        apos:entity_iw4::Trajectory::default(), entnum:sim::GENTITY_RESERVED_COUNT,
        launch_time:0, spawn_time_ms:0, detonate_at_ms:None, cleanup_at_ms:1000,
        travel_distance:0.0, live:true, stuck_pane:None, grounded:false, guide:sim::MissileGuide::default(),
    });
    let mut changed = bench::snapshot(2, 3, 6, 7, 1);
    changed.projectiles=baseline.projectiles.clone(); changed.projectiles[0].origin[0]=64.0;
    let mut unchanged = baseline.clone(); unchanged.tick = Tick(3);
    let mut encoder = SnapshotEncoder::new(); encoder.adopt_baseline(&baseline);
    let bad_delta = encoder.encode(&changed);
    encoder.adopt_baseline(&baseline);
    let recovery = encoder.encode(&unchanged);
    let mut out = WireWriter::with_capacity(1024); bad_delta.encode(&mut out);
    let encoded = out.finish();
    let mut rejected = 0; let mut corrupted = 0; let mut recovered_wrong = 0;
    for cut in 0..=bad_delta.payload_len() {
        let mut out = WireWriter::with_capacity(cut+8);
        let trailing=usize::from(cut==bad_delta.payload_len());
        out.put_u32(2); out.put_u32((cut+trailing) as u32); out.put_bytes(&encoded[8..8+cut]);
        if trailing==1 {out.put_u8(255);}
        let wire = out.finish();
        let bad = SnapshotDelta::decode(&mut WireReader::new(&wire)).expect("bounded delta container");
        let mut decoder = SnapshotDecoder::new(); decoder.adopt_baseline(&baseline);
        let before = decoder.last_projectile_delta().clone();
        if decoder.decode(&bad).is_err() {
            rejected += 1;
            corrupted += usize::from(*decoder.last_projectile_delta()!=before);
            let after = decoder.decode(&recovery).expect("recovery snapshot");
            recovered_wrong += usize::from(after.players!=unchanged.players || after.projectiles!=unchanged.projectiles);
        }
    }
    println!("rollback_snapshot rejected={rejected} last_delta_corrupt={corrupted} recovery_wrong={recovered_wrong}");
    let mut world_encoder = WorldObjectSyncEncoder::default();
    world_encoder.adopt_baseline(baseline.meta.world_objects.clone());
    let world_wire = world_encoder.encode(Tick(2), &changed.meta.world_objects);
    let mut world_rejected = 0; let mut world_corrupt = 0;
    for cut in 0..=world_wire.len() {
        let mut wire = world_wire[..cut].to_vec(); if cut==world_wire.len() { wire.push(255); }
        let mut decoder = WorldObjectSyncDecoder::default();
        decoder.adopt_baseline(baseline.meta.world_objects.clone());
        assert!(decoder.apply_wire(&wire).is_err(), "truncation/trailing world rejected");
        world_rejected += 1; world_corrupt += usize::from(decoder.state()!=&baseline.meta.world_objects);
        decoder.adopt_baseline(baseline.meta.world_objects.clone());
        decoder.apply_wire(&world_wire).expect("valid after bad world");
    }
    println!("rollback_world rejected={world_rejected} state_corrupt={world_corrupt}");
    let mut encoder = SnapshotEncoder::new(); encoder.adopt_baseline(&baseline);
    let bad_frame = frame_from_acked_tick(&mut encoder, &TickInput::default(), &changed, Vec::new());
    let frame_wire = bad_frame.to_bytes(); let mut frame_rejected = 0; let mut frame_corrupt = 0;
    for cut in 0..frame_wire.len() {
        let mut decoder = WorldObjectSyncDecoder::default(); decoder.adopt_baseline(baseline.meta.world_objects.clone());
        if Frame::decode(&mut WireReader::new(&frame_wire[..cut]), &mut decoder).is_err() {
            frame_rejected += 1; frame_corrupt += usize::from(decoder.state()!=&baseline.meta.world_objects);
        }
    }
    let transaction = include_str!("../src/transport/frame.rs").contains("read_snapshot_meta(input)?");
    assert!(rejected>0 && world_rejected>0 && frame_rejected>0);
    if transaction {
        assert_eq!((corrupted,recovered_wrong,world_corrupt,frame_corrupt),(0,0,0,0));
    } else { assert!(recovered_wrong>0 && world_corrupt>0 && frame_corrupt>0, "negative control master must observe corruption"); }
    println!("rollback_frame rejected={frame_rejected} state_corrupt={frame_corrupt}");
    println!("rollback_probe=ok transactional={transaction} public_production=true live_socket=false");
}
fn main() {
    if std::env::args().nth(1).as_deref()==Some("probe") {run_probe();} else {bench::run();}
}
