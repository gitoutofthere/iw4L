//! Temporary public-API client ingress probe, with synthetic snapshots only.
use entity_iw4::{EntityState, Trajectory};
use master_protocol::MemberId;
use net::transport::protocol::{
    ClientPacket, ConnectionId, ContentFingerprint, HandshakeHello, PacketHeader,
    ProtocolLimits, ServerPacket, decode_client_packet,
};
use net::transport::udp_session::{RelayMailbox, UdpClientLink};
use net::{SnapshotEncoder, frame_from_acked_tick};
use playerstate_iw4::PlayerState;
use sim::{
    ClientId, ClientSnapshotMeta, DroppedItemAmmo, EntityKernelOccupiedSnapshot,
    EntityKernelSlotSnapshot, EntityRelations, EntityRunKind, LifeSequence,
    MissileGuide, ProjectileId, ProjectileState, ScriptModelId, ScriptMoverGentity,
    Snapshot, Tick, TickInput,
};

fn basic() -> Snapshot {
    let mut s = Snapshot::unpublished(Tick(1));
    let mut ps = PlayerState::ZERO;
    ps.weapons[0] = 1;
    ps.weapon = 1;
    ps.weapon_primary = 1;
    ps.off_hand_index = 2;
    s.players.push((ClientId(1), ps));
    s.meta.clients.push((ClientId(1), ClientSnapshotMeta::default()));
    s
}

fn typed(kind: EntityRunKind, e_type: i32) -> Snapshot {
    let mut s = basic();
    let number = sim::GENTITY_RESERVED_COUNT;
    s.meta.entity_kernel.high_water = number + 1;
    s.meta.entity_kernel.level_time_ms = 50;
    s.meta.entity_kernel.frame_serial = 1;
    s.meta.entity_kernel.slots.push(EntityKernelSlotSnapshot {
        generation: 1,
        occupied: Some(EntityKernelOccupiedSnapshot {
            kind, linked: true, relations: EntityRelations::default(),
            next_think_ms: None, transient_event_time_ms: None,
        }),
        freed_at_ms: 0,
    });
    s.meta.entities.push(EntityState { number, e_type, ..Default::default() });
    s
}

fn item() -> Snapshot {
    let mut s = typed(EntityRunKind::Item, entity_iw4::ET_ITEM);
    s.meta.item_ammo.push(DroppedItemAmmo {
        entnum: sim::GENTITY_RESERVED_COUNT, ..Default::default()
    });
    s
}

fn mover() -> Snapshot {
    let mut s = typed(EntityRunKind::ScriptMover, entity_iw4::ET_SCRIPTMOVER);
    s.meta.script_movers.push(ScriptMoverGentity {
        id: ScriptModelId::from_wire(1), state: s.meta.entities[0], ..Default::default()
    });
    s
}

fn missile() -> Snapshot {
    let mut s = typed(EntityRunKind::Missile, entity_iw4::ET_MISSILE);
    s.projectiles.push(ProjectileState {
        id: ProjectileId(1), owner: ClientId(1), owner_life: LifeSequence(0), weapon: 1,
        origin: [0.0; 3], velocity: [0.0; 3], pos: Trajectory::default(),
        apos: Trajectory::default(), entnum: sim::GENTITY_RESERVED_COUNT,
        launch_time: 0, spawn_time_ms: 0, detonate_at_ms: None,
        cleanup_at_ms: 1000, travel_distance: 0.0, live: true,
        stuck_pane: None, grounded: false, guide: MissileGuide::default(),
    });
    s
}

fn packet(s: &Snapshot, seq: u32, baseline_seq: u32) -> Vec<u8> {
    let mut encoder = SnapshotEncoder::new();
    let frame = frame_from_acked_tick(&mut encoder, &TickInput::default(), s, Vec::new());
    ServerPacket::Snapshot {
        header: PacketHeader { connection: ConnectionId(1), sequence: seq, ack: 0, epoch: 0 },
        snapshot_seq: seq, baseline_seq, payload: frame.to_bytes(),
    }.to_bytes()
}

fn new_link() -> (UdpClientLink, RelayMailbox) {
    let mailbox = RelayMailbox::new(8);
    let mut link = UdpClientLink::relay(
        HandshakeHello::current(ContentFingerprint::default()), mailbox.clone()
    );
    link.note_accept(ConnectionId(1), ClientId(1));
    (link, mailbox)
}

fn inject(mail: &RelayMailbox, bytes: Vec<u8>) {
    mail.push_inbound(MemberId([0; 16]), bytes).unwrap();
}

fn assert_ack(mail: &RelayMailbox, seq: u32) {
    let out = mail.take_outbound();
    assert_eq!(out.len(), 1);
    match decode_client_packet(&out[0].1, &ProtocolLimits::default()).unwrap() {
        ClientPacket::SnapshotAck { snapshot_seq, .. } => assert_eq!(snapshot_seq, seq),
        _ => panic!("unexpected outbound packet"),
    }
}

fn main() {
    let iterations: usize = std::env::args().nth(1).unwrap_or("100".into()).parse().unwrap();
    let valid = [basic(), item(), mover(), missile()];
    let mut invalid = Vec::new();
    let mut s = basic(); s.meta.clients.clear(); invalid.push(("missing player meta", s));
    let mut s = item(); s.meta.item_ammo.clear(); invalid.push(("missing item ammo", s));
    let mut s = item(); s.meta.entities.push(s.meta.entities[0]); invalid.push(("duplicate item", s));
    let mut s = item(); s.meta.entity_kernel.slots[0].occupied.as_mut().unwrap().kind = EntityRunKind::General;
    invalid.push(("item kind", s));
    let mut s = mover(); s.meta.entities.clear(); invalid.push(("missing mover presentation", s));
    let mut s = mover(); s.meta.script_movers.push(s.meta.script_movers[0]); invalid.push(("duplicate mover", s));
    let mut s = mover(); s.meta.entities[0].tr_base[0] = f32::NAN;
    s.meta.script_movers[0].state.tr_base[0] = f32::NAN; invalid.push(("NaN mover equality", s));
    let mut s = missile(); s.meta.entity_kernel.slots[0].occupied.as_mut().unwrap().kind = EntityRunKind::General;
    invalid.push(("missile kind", s));
    let mut s = item(); s.meta.entities[0].tr_type = 4; invalid.push(("unsupported item trajectory", s));
    let mut s = item(); s.meta.entities[0].apos_tr_type = 9; invalid.push(("position-only trajectory", s));
    let mut s = missile(); s.projectiles[0].pos.tr_type = 4; invalid.push(("unsupported projectile trajectory", s));
    let mut s = basic(); s.players[0].1.weapons[1] = 999; invalid.push(("unknown owned weapon", s));
    let mut s = basic(); s.players[0].1.weapon = 999; invalid.push(("unknown current weapon", s));
    let mut s = basic(); s.players[0].1.weapon_primary = 999; invalid.push(("unknown primary weapon", s));
    let mut s = basic(); s.players[0].1.off_hand_index = 999; invalid.push(("unknown offhand weapon", s));
    for _ in 0..iterations {
        for s in &valid {
            assert_eq!(sim::snapshot_fault(s, 3), None);
            let (mut link, mail) = new_link();
            inject(&mail, packet(s, 1, 0));
            let received = link.recv_ticks(3).unwrap();
            assert_eq!(received.len(), 1);
            assert!(link.has_applied_snapshot());
            assert_ack(&mail, 1);
            let mut world = sim::SimWorld::new();
            world.adopt_prediction_snapshot(&received[0].snapshot, ClientId(1));
        }
        for (label, s) in &invalid {
            assert!(sim::snapshot_fault(s, 3).is_some(), "{label}");
            let (mut link, mail) = new_link();
            inject(&mail, packet(s, 1, 0));
            assert!(link.recv_ticks(3).unwrap().is_empty(), "{label}");
            assert!(!link.has_applied_snapshot(), "{label}");
            assert!(mail.take_outbound().is_empty(), "{label}");
            // A subsequent packet cannot decode against the refused sequence.
            inject(&mail, packet(&valid[0], 2, 1));
            assert!(link.recv_ticks(3).unwrap().is_empty(), "{label}");
            assert!(mail.take_outbound().is_empty(), "{label}");
            // Full resend of the same sequence is accepted, proving it was not cached.
            inject(&mail, packet(&valid[0], 1, 0));
            assert_eq!(link.recv_ticks(3).unwrap().len(), 1, "{label}");
            assert_ack(&mail, 1);
        }
        let mut empty = basic(); empty.players[0].1 = PlayerState::ZERO;
        empty.players[0].1.off_hand_index = -1;
        let (mut link, mail) = new_link();
        inject(&mail, packet(&empty, 1, 0));
        assert_eq!(link.recv_ticks(0).unwrap().len(), 1);
        assert_ack(&mail, 1);
        link.reset_match(); link.note_accept(ConnectionId(1), ClientId(1));
        inject(&mail, packet(&valid[0], 1, 0));
        assert!(link.recv_ticks(2).unwrap().is_empty()); // offhand id 2 is beyond this new bound
        inject(&mail, packet(&valid[0], 1, 0));
        assert_eq!(link.recv_ticks(3).unwrap().len(), 1);
        assert_ack(&mail, 1);
        let (mut link, mail) = new_link();
        inject(&mail, packet(&valid[0], 1, 0));
        inject(&mail, packet(&invalid[0].1, 2, 0));
        inject(&mail, packet(&valid[0], 3, 0));
        let received = link.recv_ticks(3).unwrap();
        assert_eq!(received.len(), 2, "invalid row must not discard valid batch rows");
        let outbound = mail.take_outbound();
        let acks: Vec<u32> = outbound.iter().map(|(_, bytes)| {
            match decode_client_packet(bytes, &ProtocolLimits::default()).unwrap() {
                ClientPacket::SnapshotAck { snapshot_seq, .. } => snapshot_seq,
                _ => panic!("unexpected batch outbound"),
            }
        }).collect();
        assert_eq!(acks, vec![1, 3]);
        let mut weapons = [0; 15]; weapons[0] = 1; weapons[1] = 999;
        assert_eq!(input_iw4::weapon_select::cycle_weapon(&weapons, 1, 0, 0, true, |_| 0), Some(999));
    }
    for t in -20..40 {
        let supported = entity_iw4::trajectory_type_supported(t);
        if supported {
            let tr = Trajectory { tr_type: t, ..Default::default() };
            entity_iw4::evaluate_trajectory(&tr, 50);
            entity_iw4::evaluate_trajectory_delta(&tr, 50);
        }
    }
    println!("client ingress probe: iterations={iterations}, valid fixtures={}, invalid fixtures={}; decode, adoption, no ack/cache on rejection, same-sequence full resend and catalog refresh passed", valid.len(), invalid.len());
}
