//! Temporary probe: fresh versus reused decoder setup using unchanged APIs.
//! Each process alternates five batches, so setup costs have paired repeats.
//! No sockets, game data or production optimization is included.

use std::hint::black_box;
use std::time::Instant;
use net::{SnapshotDecoder, SnapshotEncoder, WorldObjectSyncDecoder};
use playerstate_iw4::PlayerState;
use sim::{ClientId, GlassCause, GlassPieceSnapshot, GlassPieceState, LifeSequence,
          MissileGuide, ProjectileId, ProjectileState, Snapshot, Tick};

fn baseline(players: usize, projectiles: usize, glass: usize, phase: usize) -> Snapshot {
    let mut s = Snapshot::unpublished(Tick(phase as u32));
    s.players = (0..players).map(|i| {
        let mut p = PlayerState::ZERO;
        p.origin = [i as f32, phase as f32, 16.0];
        (ClientId(i as u32), p)
    }).collect();
    s.projectiles = (0..projectiles).map(|i| ProjectileState {
        id: ProjectileId(i as u32), owner: ClientId(0), owner_life: LifeSequence(0), weapon: 1,
        origin: [i as f32, 0.0, 0.0], velocity: [0.0; 3],
        pos: entity_iw4::Trajectory::default(), apos: entity_iw4::Trajectory::default(),
        entnum: 64 + i as i32, launch_time: 0, spawn_time_ms: 0,
        detonate_at_ms: None, cleanup_at_ms: 1000, travel_distance: 0.0, live: true,
        stuck_pane: None, grounded: false, guide: MissileGuide::default(),
    }).collect();
    s.meta.world_objects.as_of_ms = phase as i32;
    s.meta.world_objects.glass_pieces = (0..glass).map(|i| (i as u32, GlassPieceSnapshot {
        state: GlassPieceState::Intact, revision: phase as u32, last_state_change_time: 0,
        shatter_seed: None, deterministic_seed: i as u64, cause: GlassCause::Impact,
    })).collect();
    s
}

fn setup(snapshot: &Snapshot, decoder: &mut SnapshotDecoder, world: &mut WorldObjectSyncDecoder) {
    decoder.adopt_baseline(snapshot);
    world.adopt_baseline(snapshot.meta.world_objects.clone());
    black_box(&*decoder);
    black_box(&*world);
}

fn elapsed_us(iterations: usize, f: impl FnOnce()) -> f64 {
    let start = Instant::now();
    f();
    start.elapsed().as_secs_f64() * 1e6 / iterations as f64
}

fn main() {
    let args: Vec<usize> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let get = |i: usize, d: usize| args.get(i).copied().unwrap_or(d);
    let (players, projectiles, glass, iterations) = (get(0, 18), get(1, 0), get(2, 300), get(3, 100_000));
    let snapshots: Vec<_> = (0..4).map(|p| baseline(players, projectiles, glass, p)).collect();
    // Compare actual public decode results after changing and clearing a baseline.
    let mut reused = SnapshotDecoder::new();
    let mut reused_world = WorldObjectSyncDecoder::default();
    for pass in 0..32 {
        let s = &snapshots[pass % 4];
        let mut fresh = SnapshotDecoder::new();
        let mut fresh_world = WorldObjectSyncDecoder::default();
        if pass % 7 == 0 {
            reused.reset();
            reused_world.reset();
        } else {
            setup(s, &mut fresh, &mut fresh_world);
            setup(s, &mut reused, &mut reused_world);
        }
        let mut encoder = SnapshotEncoder::new();
        if pass % 7 != 0 { encoder.adopt_baseline(s); }
        let delta = encoder.encode(&snapshots[(pass + 1) % 4]);
        assert_eq!(fresh.decode(&delta).unwrap(), reused.decode(&delta).unwrap());
        assert_eq!(fresh_world.state(), reused_world.state());
    }
    println!("players={players} projectiles={projectiles} glass={glass} iterations={iterations}; public decode/reset equality passed");
    for batch in 1..=5 {
        let order = if batch % 2 == 1 { ["fresh", "reuse"] } else { ["reuse", "fresh"] };
        for mode in order {
            let cost = elapsed_us(iterations, || {
                if mode == "fresh" {
                    for i in 0..iterations {
                        let mut d = SnapshotDecoder::new();
                        let mut w = WorldObjectSyncDecoder::default();
                        setup(black_box(&snapshots[i % 4]), &mut d, &mut w);
                    }
                } else {
                    let mut d = SnapshotDecoder::new();
                    let mut w = WorldObjectSyncDecoder::default();
                    for i in 0..iterations { setup(black_box(&snapshots[i % 4]), &mut d, &mut w); }
                }
            });
            println!("setup batch={batch} mode={mode}: {cost:.6} us/packet");
        }
        let cost = elapsed_us(iterations, || {
            for _ in 0..iterations { black_box((SnapshotDecoder::new(), WorldObjectSyncDecoder::default())); }
        });
        println!("constructor batch={batch}: {cost:.6} us/packet");
    }
}
