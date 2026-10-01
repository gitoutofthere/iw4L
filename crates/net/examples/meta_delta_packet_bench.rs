//! Temporary metadata byte-delta experiment including the existing zstd packet envelope.
use std::collections::BTreeMap;
use std::time::Instant;
use net::{Frame, SnapshotEncoder, WorldObjectSyncDecoder, frame_from_acked_tick};
use net::transport::protocol::{ServerPacket, PacketHeader, ConnectionId, ProtocolLimits, decode_server_packet};
use net::transport::wire::{WireError, WireReader, WireWriter};
use playerstate_iw4::PlayerState;
use sim::{ClientId, ClientSnapshotMeta, EntityKernelOccupiedSnapshot, EntityKernelSlotSnapshot,
    EntityRelations, EntityRunKind, GlassCause, GlassPieceSnapshot, GlassPieceState, Snapshot, Tick, TickInput};

fn snapshot(tick: u32, players: usize, entities: usize, glass: usize, churn: usize) -> Snapshot {
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


#[derive(Clone)]
struct Parts([Vec<u8>; 5]);
fn parts(frame: &Frame, raw: &[u8]) -> Parts {
    let s = frame.section_bytes();
    assert_eq!(s.total, raw.len());
    assert_eq!(s.named_sum(), raw.len());
    let e = s.header + s.snapshot_delta + s.meta.match_header + s.meta.events
        + s.meta.aliases + s.meta.entity_dobjs + s.meta.corpses;
    let e_end = e + s.meta.entities;
    let k = e_end + s.meta.script_movers;
    let k_end = k + s.meta.entity_kernel;
    Parts([raw[..e].to_vec(), raw[e..e_end].to_vec(), raw[e_end..k].to_vec(),
           raw[k..k_end].to_vec(), raw[k_end..].to_vec()])
}
fn delta(base: Option<&[u8]>, now: &[u8]) -> Vec<u8> {
    let mut full = WireWriter::new();
    full.put_u8(2); full.put_u32(now.len() as u32); full.put_bytes(now);
    let Some(base) = base.filter(|b| b.len() == now.len()) else { return full.finish(); };
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for i in 0..now.len() {
        if base[i] == now[i] { continue; }
        if let Some(last) = runs.last_mut() && i - last.1 <= 8 { last.1 = i+1; }
        else { runs.push((i, i+1)); }
    }
    let mut out = WireWriter::new();
    out.put_u8(if runs.is_empty() { 0 } else { 1 }); out.put_u32(now.len() as u32);
    if !runs.is_empty() {
        out.put_u32(runs.len() as u32);
        for (begin, end) in runs { out.put_u32(begin as u32); out.put_u32((end-begin) as u32); out.put_bytes(&now[begin..end]); }
    }
    if out.len() < full.len() { out.finish() } else { full.finish() }
}
fn apply(base: Option<&[u8]>, wire: &[u8], limit: usize) -> Result<Vec<u8>, WireError> {
    let bad = WireError::Malformed;
    let mut r = WireReader::new(wire);
    let tag = r.get_u8()?;
    let len = r.get_u32()? as usize;
    if len > limit { return Err(bad("prototype section exceeds limit")); }
    let mut out = match tag {
        2 => {
            if r.remaining() != len { return Err(bad("prototype full length")); }
            let mut out = vec![0; len]; r.get_bytes(&mut out)?; out
        }
        0 | 1 => {
            let b = base.filter(|b| b.len() == len).ok_or(bad("prototype baseline"))?;
            b.to_vec()
        }
        _ => return Err(bad("prototype tag")),
    };
    if tag == 1 {
        let count = r.get_u32()? as usize;
        if count > r.remaining()/8 { return Err(bad("prototype patch count")); }
        let mut previous = 0;
        for _ in 0..count {
            let start = r.get_u32()? as usize; let size = r.get_u32()? as usize;
            if size == 0 || start < previous || start > len || size > len-start { return Err(bad("prototype patch range")); }
            r.get_bytes(&mut out[start..start+size])?; previous = start+size;
        }
    }
    if !r.is_empty() { return Err(bad("prototype trailing bytes")); }
    Ok(out)
}
fn pack(now: &Parts, base: Option<&Parts>) -> Vec<u8> {
    let mut out = WireWriter::new();
    for i in 0..5 {
        let row = if i == 1 || i == 3 { delta(base.map(|b| b.0[i].as_slice()), &now.0[i]) } else { now.0[i].clone() };
        out.put_u32(row.len() as u32); out.put_bytes(&row);
    }
    out.finish()
}
fn unpack(wire: &[u8], base: Option<&Parts>, limit: usize) -> Result<Parts, WireError> {
    if wire.len() > limit { return Err(WireError::Malformed("prototype packet limit")); }
    let mut r = WireReader::new(wire);
    let mut rows: [Vec<u8>; 5] = Default::default();
    let mut total = 0usize;
    for i in 0..5 {
        let len = r.get_u32()? as usize;
        if len > r.remaining() || len > limit { return Err(WireError::Malformed("prototype chunk length")); }
        let mut row = vec![0; len]; r.get_bytes(&mut row)?;
        rows[i] = if i == 1 || i == 3 { apply(base.map(|b| b.0[i].as_slice()), &row, limit)? } else { row };
        total = total.checked_add(rows[i].len()).filter(|n| *n <= limit)
            .ok_or(WireError::Malformed("prototype reconstructed size"))?;
    }
    if !r.is_empty() { return Err(WireError::Malformed("prototype container trailing")); }
    Ok(Parts(rows))
}
fn main() {
    let args: Vec<usize> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let get = |i, d| args.get(i).copied().unwrap_or(d);
    let (players, entities, glass, churn, ticks, lag) = (get(0,18),get(1,400),get(2,300),get(3,1),get(4,1000),get(5,3));
    let limit = net::transport::protocol::ProtocolLimits::default().max_packet_bytes as usize;
    let mut history: BTreeMap<u32, (Snapshot, Parts)> = BTreeMap::new();
    let mut encoder = SnapshotEncoder::new();
    let input = TickInput::default();
    let mut bytes: Vec<(usize, usize)> = Vec::new();
    let mut packet_bytes: Vec<(usize, usize)> = Vec::new();
    let mut costs: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    let mut rejected = 0;
    for tick in 1..=ticks as u32 {
        let current = snapshot(tick, players, entities, glass, churn);
        let baseline = history.get(&tick.saturating_sub(lag as u32));
        if let Some((s,_)) = baseline { encoder.adopt_baseline(s); } else { encoder.reset(); }
        let start = Instant::now();
        let frame = frame_from_acked_tick(&mut encoder, &input, &current, Vec::new());
        let raw = frame.to_bytes();
        costs.entry("full encode").or_default().push(start.elapsed().as_secs_f64()*1e6);
        assert!(raw.len()+64 < limit, "synthetic frame leaves space for packet envelope");
        let spans = parts(&frame, &raw);
        let start = Instant::now();
        let wire = pack(&spans, baseline.map(|(_,p)| p));
        costs.entry("prototype extra encode").or_default().push(start.elapsed().as_secs_f64()*1e6);
        // Use the production compression/framing path on each payload. The new
        // metadata payload is interpreted by this example only after packet decode.
        let mut encoded_packet_sizes = [0; 2];
        let order = if tick % 2 == 1 { [("full", &raw, 0), ("prototype", &wire, 1)] }
            else { [("prototype", &wire, 1), ("full", &raw, 0)] };
        for (name, payload, index) in order {
            let packet = ServerPacket::Snapshot {
                header: PacketHeader { connection: ConnectionId(1), sequence: tick, ack: 0, epoch: 1 },
                baseline_seq: tick.saturating_sub(lag as u32), snapshot_seq: tick,
                payload: payload.clone(),
            };
            let start = Instant::now();
            let encoded = packet.to_bytes();
            costs.entry(if name == "full" { "full ServerPacket encode" } else { "prototype ServerPacket encode" })
                .or_default().push(start.elapsed().as_secs_f64()*1e6);
            let start = Instant::now();
            let parsed = decode_server_packet(&encoded, &ProtocolLimits::default()).unwrap();
            costs.entry(if name == "full" { "full ServerPacket decode" } else { "prototype ServerPacket decode" })
                .or_default().push(start.elapsed().as_secs_f64()*1e6);
            match parsed { ServerPacket::Snapshot { payload: decoded_payload, .. } => assert_eq!(decoded_payload, *payload), _ => panic!("snapshot tag") }
            encoded_packet_sizes[index] = encoded.len();
        }
        packet_bytes.push((encoded_packet_sizes[0], encoded_packet_sizes[1]));
        let start = Instant::now();
        let decoded = unpack(&wire, baseline.map(|(_,p)| p), limit).unwrap();
        let joined: Vec<_> = decoded.0.iter().flatten().copied().collect();
        costs.entry("prototype extra decode").or_default().push(start.elapsed().as_secs_f64()*1e6);
        assert_eq!(joined, raw);
        let mut old_world = WorldObjectSyncDecoder::default();
        let mut new_world = WorldObjectSyncDecoder::default();
        if let Some((s,_)) = baseline { old_world.adopt_baseline(s.meta.world_objects.clone()); new_world.adopt_baseline(s.meta.world_objects.clone()); }
        let start = Instant::now();
        let parsed = Frame::decode(&mut WireReader::new(&raw), &mut old_world).unwrap();
        costs.entry("full Frame decode").or_default().push(start.elapsed().as_secs_f64()*1e6);
        assert_eq!(Frame::decode(&mut WireReader::new(&joined), &mut new_world).unwrap(), parsed);
        if tick <= 5 {
            for cut in [0, 1, wire.len()/2, wire.len()-1] { assert!(unpack(&wire[..cut], baseline.map(|(_,p)|p), limit).is_err()); rejected += 1; }
            let mut trailing = wire.clone(); trailing.push(0); assert!(unpack(&trailing, baseline.map(|(_,p)|p), limit).is_err()); rejected += 1;
        }
        bytes.push((raw.len(),wire.len()));
        history.insert(tick,(current,decoded));
        while history.len() > 64 { let first = *history.keys().next().unwrap(); history.remove(&first); }
    }
    println!("players={players} entities={entities} glass={glass} churn={churn} ticks={ticks} lag={lag}; exact bytes/Frame equality passed; malformed rejected={rejected}");
    let old = bytes.iter().map(|p| p.0).sum::<usize>() as f64/ticks as f64;
    let new = bytes.iter().map(|p| p.1).sum::<usize>() as f64/ticks as f64;
    println!("prototype bytes: full mean {old:.2} max {} | candidate mean {new:.2} max {} | ratio {:.6}", bytes.iter().map(|p|p.0).max().unwrap(), bytes.iter().map(|p|p.1).max().unwrap(),new/old);
    let old_packet = packet_bytes.iter().map(|p| p.0).sum::<usize>() as f64/ticks as f64;
    let new_packet = packet_bytes.iter().map(|p| p.1).sum::<usize>() as f64/ticks as f64;
    println!("prototype packet bytes: full mean {old_packet:.2} max {} | candidate mean {new_packet:.2} max {} | ratio {:.6}", packet_bytes.iter().map(|p|p.0).max().unwrap(), packet_bytes.iter().map(|p|p.1).max().unwrap(),new_packet/old_packet);
    for (name,values) in &mut costs {
        values.sort_by(f64::total_cmp);
        println!("prototype step {name}: mean {:.3} p50 {:.3} p95 {:.3} us", values.iter().sum::<f64>()/values.len() as f64,values[values.len()/2],values[values.len()*95/100]);
    }
}
