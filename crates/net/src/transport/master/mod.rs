use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use master_protocol::{
    ALPN, AdmissionFailure, AdvertId, ContentFlags, ControlFrame, ControlHello, ControlRequest,
    ControlResponse, EndpointRole, MAX_BOOTSTRAP_STREAM_BYTES, MAX_CONCURRENT_BOOTSTRAP,
    MAX_RELAY_UNI_STREAMS, MemberId, RelayDatagram, RequestBody, ResponseBody, RoomView,
    SESSION_IDLE, SESSION_KEEP_ALIVE, SessionCloseReason, decode_relay, decode_relay_stream,
    decode_stream_payload, encode_relay, encode_relay_stream, encode_stream_frame,
    stream_frame_len,
};
use quinn::crypto::rustls::QuicClientConfig;
use rustls::pki_types::CertificateDer;
use rustls_platform_verifier::ConfigVerifierExt;
use tokio_util::sync::CancellationToken;

use crate::authority::runtime::AuthorityWorld;
use crate::session_core::{
    HostMatchApply, HostMatchCore, HostMatchEffect, HostMatchEvent, HostWorldReady, SessionCore,
    SessionEvent,
};
use crate::transport::bootstrap::{
    BootstrapAck, BootstrapLane, BootstrapMessage, decode_bootstrap, epoch_applies,
};
use crate::transport::fragment::{Fragmenter, Reassembler};
use crate::transport::protocol::{ContentFingerprint, HandshakeHello, MatchDescriptor};
use crate::transport::udp_session::{RelayMailbox, UdpAuthorityHub, UdpClientLink};

mod session;

use session::{
    browser_worker, handshake_for_match, publish_failed, refresh_relay_authority_hello, spawn_host,
    spawn_join,
};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, Error>;

pub const CONTENT_IW4: u8 = 1 << 0;
pub const CONTENT_IW5: u8 = 1 << 1;
pub const CONTENT_T5: u8 = 1 << 2;

pub const fn content_inventory(iw4: bool, iw5: bool, t5: bool) -> ContentFlags {
    ContentFlags(
        (if iw4 { CONTENT_IW4 } else { 0 })
            | (if iw5 { CONTENT_IW5 } else { 0 })
            | (if t5 { CONTENT_T5 } else { 0 }),
    )
}

pub fn content_required_by_map(map: &str) -> Result<ContentFlags> {
    let namespace = map
        .split_once(':')
        .map_or("iw4", |(namespace, _)| namespace);
    Ok(ContentFlags(match namespace {
        "iw4" => CONTENT_IW4,
        "iw5" => CONTENT_IW5,
        "t5" => CONTENT_T5,
        other => return Err(format!("unknown content namespace `{other}` in map `{map}`").into()),
    }))
}

pub fn content_names(flags: ContentFlags) -> String {
    let mut names = Vec::new();
    if flags.0 & CONTENT_IW4 != 0 {
        names.push("iw4");
    }
    if flags.0 & CONTENT_IW5 != 0 {
        names.push("iw5");
    }
    if flags.0 & CONTENT_T5 != 0 {
        names.push("t5");
    }
    if flags.0 & !(CONTENT_IW4 | CONTENT_IW5 | CONTENT_T5) != 0 {
        names.push("unknown");
    }
    names.join(",")
}

const RELAY_PACKET_BYTES: usize = crate::transport::protocol::MAX_PACKET_BYTES as usize;
const HOST_DATA_CAP: usize = 64;
const HOST_CONTROL_CAP: usize = 32;

const BOOTSTRAP_PRIORITY: i32 = -32;
const IO_DEADLINE: Duration = Duration::from_secs(8);
const LEAVE_DRAIN: Duration = Duration::from_millis(500);
const QUEUE_POLL: Duration = Duration::from_millis(5);
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(1);

fn blocking_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn endpoint_build() -> String {
    format!(
        "iw4l-net/{} game={} master={}",
        env!("CARGO_PKG_VERSION"),
        crate::PROTOCOL_VERSION,
        master_protocol::PROTOCOL_VERSION
    )
}

fn relay_fragmenter() -> Fragmenter {
    Fragmenter::new(master_protocol::MAX_OPAQUE_PAYLOAD, RELAY_PACKET_BYTES)
}

fn relay_reassembler() -> Reassembler {
    Reassembler::new(master_protocol::MAX_OPAQUE_PAYLOAD, RELAY_PACKET_BYTES)
}

fn take_commands(commands: &Mutex<Vec<MasterBridgeCommand>>) -> Vec<MasterBridgeCommand> {
    let mut queue = commands.lock().expect("master command queue poisoned");
    std::mem::take(&mut *queue)
}

fn classify_try_send<T>(
    result: std::result::Result<(), tokio::sync::mpsc::error::TrySendError<T>>,
    operation: &'static str,
) -> std::result::Result<(), QueueLoss> {
    match result {
        Ok(()) => Ok(()),
        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => Err(QueueLoss::Full { operation }),
        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
            Err(QueueLoss::Closed { operation })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QueueLoss {
    Full { operation: &'static str },
    Closed { operation: &'static str },
}

impl fmt::Display for QueueLoss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Full { operation } => write!(f, "{operation} queue full"),
            Self::Closed { operation } => write!(f, "{operation} queue closed"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionIdentity {
    pub attempt_id: u64,
    pub room_id: AdvertId,
    pub member_id: MemberId,
    pub epoch: u32,
}

impl SessionIdentity {
    pub const fn unassigned(attempt_id: u64) -> Self {
        Self {
            attempt_id,
            room_id: AdvertId([0; 16]),
            member_id: MemberId([0; 16]),
            epoch: 0,
        }
    }

    pub fn match_key(&self) -> frame::MatchKey {
        if self.epoch == 0 || self.room_id.0 == [0; 16] {
            frame::MatchKey::NONE
        } else {
            frame::MatchKey::new(self.room_id.0, self.epoch)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportFault {
    pub operation: &'static str,
    pub role: &'static str,
    pub source: String,
    pub close_reason: Option<String>,
}

impl TransportFault {
    pub fn new(operation: &'static str, role: &'static str, source: impl Into<String>) -> Self {
        Self {
            operation,
            role,
            source: source.into(),
            close_reason: None,
        }
    }

    fn with_connection(
        operation: &'static str,
        role: &'static str,
        source: impl fmt::Display,
        connection: &quinn::Connection,
    ) -> Self {
        Self {
            operation,
            role,
            source: source.to_string(),
            close_reason: connection.close_reason().map(|error| error.to_string()),
        }
    }
}

impl fmt::Display for TransportFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.role, self.operation, self.source)?;
        if let Some(close) = &self.close_reason {
            write!(f, " (quic close: {close})")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MasterLifecycleFact {
    MemberLeft {
        member_id: MemberId,
    },
    SessionClosed {
        reason: SessionCloseReason,
    },

    AdmissionFailed {
        member_id: MemberId,
        reason: AdmissionFailure,
    },
}

fn observe_live_start(in_match: bool, start_nonce: u32) -> bool {
    in_match && start_nonce != 0
}

fn push_fact(facts: &Mutex<Vec<MasterLifecycleFact>>, fact: MasterLifecycleFact) {
    facts.lock().expect("master facts poisoned").push(fact);
}

fn forget_relay_member(
    member_id: MemberId,
    members: &mut HashSet<MemberId>,
    skip_voters: &mut HashSet<MemberId>,
    map_ready: &Mutex<HashSet<MemberId>>,
    facts: &Mutex<Vec<MasterLifecycleFact>>,
) -> bool {
    if !members.remove(&member_id) {
        return false;
    }
    skip_voters.remove(&member_id);
    map_ready
        .lock()
        .expect("bootstrap readiness poisoned")
        .remove(&member_id);
    push_fact(facts, MasterLifecycleFact::MemberLeft { member_id });
    true
}

#[derive(Clone, Debug)]
struct MasterTarget {
    address: String,
    server_name: String,
    ca_cert: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct HostConfig {
    auto_start_map: bool,
    target: MasterTarget,
    name: String,
    map: String,
    mode: String,
    max_players: u8,
    requires: ContentFlags,
    have: ContentFlags,
}

#[derive(Clone, Debug)]
struct JoinConfig {
    target: MasterTarget,
    advert_id: AdvertId,
    map: String,
    mode: String,
    have: ContentFlags,
}

#[derive(Clone, Debug)]
struct BrowserConfig {
    target: MasterTarget,
    have: ContentFlags,
}

#[derive(Clone, Debug)]
enum MasterLaunchMode {
    Disabled,
    Browser(BrowserConfig),
    Host(HostConfig),
    Join(JoinConfig),
}

#[derive(Resource, Clone, Debug)]
pub struct MasterLaunchIntent(MasterLaunchMode);

impl MasterLaunchIntent {
    pub fn browser_from_env(have: ContentFlags) -> Result<Self> {
        let Ok(address) = std::env::var("IW4L_MASTER_ADDR") else {
            return Ok(Self::disabled());
        };
        let server_name = std::env::var("IW4L_MASTER_SERVER_NAME")
            .map_err(|_| "IW4L_MASTER_ADDR requires IW4L_MASTER_SERVER_NAME")?;
        Ok(Self(MasterLaunchMode::Browser(BrowserConfig {
            target: MasterTarget {
                address,
                server_name,
                ca_cert: std::env::var_os("IW4L_MASTER_CA_CERT").map(PathBuf::from),
            },
            have,
        })))
    }

    pub fn from_env_for_map(map: &str, have: ContentFlags, requires: ContentFlags) -> Result<Self> {
        let Ok(address) = std::env::var("IW4L_MASTER_ADDR") else {
            return Ok(Self(MasterLaunchMode::Disabled));
        };
        let server_name = std::env::var("IW4L_MASTER_SERVER_NAME")
            .map_err(|_| "IW4L_MASTER_ADDR requires IW4L_MASTER_SERVER_NAME")?;
        let target = MasterTarget {
            address,
            server_name,
            ca_cert: std::env::var_os("IW4L_MASTER_CA_CERT").map(PathBuf::from),
        };
        let host = std::env::var("IW4L_MASTER_HOST_NAME").ok();
        let join = std::env::var("IW4L_MASTER_JOIN").ok();
        match (host, join) {
            (Some(name), None) if !name.trim().is_empty() => {
                Ok(Self(MasterLaunchMode::Host(HostConfig {
                    auto_start_map: true,
                    target,
                    name,
                    map: map.to_owned(),
                    mode: std::env::var("IW4L_GAMETYPE").unwrap_or_else(|_| "dm".into()),
                    max_players: parse_max_players()?,
                    requires,
                    have,
                })))
            }
            (None, Some(advert_id)) => Ok(Self(MasterLaunchMode::Join(JoinConfig {
                target,
                advert_id: advert_id.parse()?,
                map: map.to_owned(),
                mode: std::env::var("IW4L_GAMETYPE").unwrap_or_else(|_| "dm".into()),
                have,
            }))),
            (Some(_), Some(_)) => {
                Err("set only one of IW4L_MASTER_HOST_NAME or IW4L_MASTER_JOIN".into())
            }
            _ => Err("IW4L_MASTER_ADDR requires IW4L_MASTER_HOST_NAME or IW4L_MASTER_JOIN".into()),
        }
    }

    pub const fn is_join(&self) -> bool {
        matches!(self.0, MasterLaunchMode::Join(_))
    }

    pub const fn enabled(&self) -> bool {
        !matches!(self.0, MasterLaunchMode::Disabled)
    }

    pub const fn disabled() -> Self {
        Self(MasterLaunchMode::Disabled)
    }

    pub const fn browser_available(&self) -> bool {
        matches!(self.0, MasterLaunchMode::Browser(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MasterAdvert {
    pub id: AdvertId,
    pub name: String,
    pub map: String,
    pub mode: String,
    pub players: u8,
    pub max_players: u8,
    pub locked: bool,
    pub in_match: bool,
    pub requires: ContentFlags,
    pub missing: ContentFlags,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MasterBrowserSnapshot {
    pub generation: u64,
    pub loading: bool,
    pub adverts: Vec<MasterAdvert>,
    pub error: Option<String>,
    pub have: ContentFlags,
}

#[derive(Resource)]
pub struct MasterBrowser {
    state: Arc<Mutex<MasterBrowserSnapshot>>,
    refresh: Arc<AtomicU64>,
    cancel: CancellationToken,
    worker: Option<JoinHandle<()>>,
}

impl MasterBrowser {
    pub fn snapshot(&self) -> MasterBrowserSnapshot {
        self.state
            .lock()
            .expect("master browser state poisoned")
            .clone()
    }

    pub fn refresh(&self) {
        self.refresh.fetch_add(1, Ordering::Relaxed);
    }
}

impl Drop for MasterBrowser {
    fn drop(&mut self) {
        self.cancel.cancel();
        let _ = self.worker.take();
    }
}

#[derive(Clone, Debug)]
pub enum MasterMenuAction {
    Refresh,
    Host {
        map: String,
        mode: String,
    },
    Join {
        advert_id: AdvertId,
        map: String,
        mode: String,
    },
    UpdateLobby {
        map: String,
        mode: String,
    },
    StartMatch {
        map: String,
        mode: String,
    },
    VoteToSkip,
    LeaveLobby,
}

#[derive(Clone, Debug)]
enum MasterBridgeCommand {
    UpdateLobby {
        map: String,
        mode: String,
    },
    StartMatch {
        map: String,
        mode: String,
    },
    VoteToSkip,
    MapLoaded {
        epoch: u32,
        map: u64,
        weapons: u64,
        classes: u64,
    },
    HostWorldReady {
        epoch: u32,
        map: u64,
        weapons: u64,
        classes: u64,
        load_key: frame::LocalLoadKey,
    },
    MatchEnded {
        match_key: frame::MatchKey,
    },
    AuthorityProgress,
    AdmitEnter {
        member_id: MemberId,
        epoch: u32,
        bootstrap_id: u32,
        connection_id: Option<u64>,
        client_id: u32,
    },
    Shutdown,
}

#[derive(Resource, Default)]
pub struct PendingMasterMenuAction(pub Option<MasterMenuAction>);

fn parse_max_players() -> Result<u8> {
    match std::env::var("IW4L_MASTER_MAX_PLAYERS") {
        Ok(raw) => {
            let value = raw.parse::<u8>()?;
            if !(2..=master_protocol::MAX_SESSION_MEMBERS).contains(&value) {
                return Err(format!("IW4L_MASTER_MAX_PLAYERS={value} must be 2..=18").into());
            }
            Ok(value)
        }
        Err(_) => Ok(master_protocol::MAX_SESSION_MEMBERS),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MasterBridgeState {
    Connecting {
        identity: SessionIdentity,
    },
    Hosting {
        identity: SessionIdentity,
        name: String,
        map: String,
        mode: String,
        members: Vec<MemberId>,
        member_names: std::collections::HashMap<MemberId, String>,
        max_players: u8,
        skip_votes: u8,
        in_match: bool,
    },
    Joining {
        identity: SessionIdentity,
    },
    Joined {
        identity: SessionIdentity,
        name: String,
        map: String,
        mode: String,
        members: Vec<MemberId>,
        member_names: std::collections::HashMap<MemberId, String>,
        max_players: u8,
        skip_votes: u8,
        in_match: bool,
    },
    Closed {
        identity: SessionIdentity,
        reason: SessionCloseReason,
    },
    Left {
        identity: SessionIdentity,
    },
    Failed {
        identity: SessionIdentity,
        error: TransportFault,
    },
}

impl MasterBridgeState {
    pub fn identity(&self) -> SessionIdentity {
        match *self {
            Self::Connecting { identity }
            | Self::Hosting { identity, .. }
            | Self::Joining { identity }
            | Self::Joined { identity, .. }
            | Self::Closed { identity, .. }
            | Self::Left { identity }
            | Self::Failed { identity, .. } => identity,
        }
    }

    pub fn map(&self) -> Option<&str> {
        match self {
            Self::Hosting { map, .. } | Self::Joined { map, .. } => Some(map),
            _ => None,
        }
    }

    pub fn mode(&self) -> Option<&str> {
        match self {
            Self::Hosting { mode, .. } | Self::Joined { mode, .. } => Some(mode),
            _ => None,
        }
    }

    pub fn members(&self) -> &[MemberId] {
        match self {
            Self::Hosting { members, .. } | Self::Joined { members, .. } => members,
            _ => &[],
        }
    }

    pub fn skip_votes(&self) -> u8 {
        match *self {
            Self::Hosting { skip_votes, .. } | Self::Joined { skip_votes, .. } => skip_votes,
            _ => 0,
        }
    }

    pub fn in_match(&self) -> bool {
        match *self {
            Self::Hosting { in_match, .. } | Self::Joined { in_match, .. } => in_match,
            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Failed { .. } | Self::Closed { .. } | Self::Left { .. }
        )
    }
}

#[derive(Resource)]
pub struct MasterBridge {
    state: Arc<Mutex<MasterBridgeState>>,
    commands: Arc<Mutex<Vec<MasterBridgeCommand>>>,
    installed_load: Arc<Mutex<Option<frame::LocalLoadKey>>>,
    mailbox: RelayMailbox,
    bootstrap: Arc<BootstrapLane>,
    close: CancellationToken,
    facts: Arc<Mutex<Vec<MasterLifecycleFact>>>,
    incarnation: u64,
    worker: Option<JoinHandle<()>>,
}

impl MasterBridge {
    pub fn state(&self) -> MasterBridgeState {
        self.state.lock().expect("master state poisoned").clone()
    }

    pub fn fail(&self, reason: &str) {
        publish_failed(
            &self.state,
            self.state().identity(),
            TransportFault::new("gameplay", "local", reason),
        );
        self.request_close();
    }

    fn request_close(&self) {
        self.send(MasterBridgeCommand::Shutdown);
        self.close.cancel();
    }

    pub fn mailbox(&self) -> RelayMailbox {
        self.mailbox.clone()
    }

    fn send(&self, command: MasterBridgeCommand) {
        self.commands
            .lock()
            .expect("master command queue poisoned")
            .push(command);
    }

    pub fn set_installed_load(&self, load: Option<frame::LocalLoadKey>) {
        *self.installed_load.lock().expect("installed load poisoned") = load;
    }

    pub fn report_map_loaded(&self, epoch: u32, descriptor: crate::MatchDescriptor) {
        self.send(MasterBridgeCommand::MapLoaded {
            epoch,
            map: descriptor.map,
            weapons: descriptor.weapons,
            classes: descriptor.classes,
        });
    }

    pub fn report_host_world_ready(
        &self,
        epoch: u32,
        descriptor: crate::MatchDescriptor,
        load_key: frame::LocalLoadKey,
    ) {
        self.send(MasterBridgeCommand::HostWorldReady {
            epoch,
            map: descriptor.map,
            weapons: descriptor.weapons,
            classes: descriptor.classes,
            load_key,
        });
    }

    pub fn report_authority_progress(&self) {
        self.send(MasterBridgeCommand::AuthorityProgress);
    }

    pub fn report_match_ended(&self, match_key: frame::MatchKey) {
        self.send(MasterBridgeCommand::MatchEnded { match_key });
    }

    pub fn start_hosted_match(&self, map: String, mode: String) {
        self.send(MasterBridgeCommand::StartMatch { map, mode });
    }

    pub fn leave(&self) {
        self.request_close();
    }

    pub fn admit_enter(
        &self,
        member_id: MemberId,
        epoch: u32,
        bootstrap_id: u32,
        connection_id: Option<u64>,
        client_id: u32,
    ) {
        self.send(MasterBridgeCommand::AdmitEnter {
            member_id,
            epoch,
            bootstrap_id,
            connection_id,
            client_id,
        });
    }

    pub fn incarnation(&self) -> u64 {
        self.incarnation
    }

    pub fn drain_facts(&self) -> Vec<MasterLifecycleFact> {
        std::mem::take(&mut *self.facts.lock().expect("master facts poisoned"))
    }
}

impl Drop for MasterBridge {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.commands.lock() {
            queue.push(MasterBridgeCommand::Shutdown);
        }
        self.close.cancel();
        let _ = self.worker.take();
    }
}

#[derive(Resource, Default)]
pub struct MasterMatchStart(Option<MasterMatchOffer>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MasterMatchOffer {
    pub map: String,
    pub mode: String,
    pub match_key: frame::MatchKey,
}

impl MasterMatchStart {
    pub fn take(&mut self) -> Option<MasterMatchOffer> {
        self.0.take()
    }
}

pub fn arm_master_bridge(
    settings: Res<frame::GameSettings>,
    intent: Res<MasterLaunchIntent>,
    role: Res<crate::RuntimeRole>,
    authority: Option<Res<AuthorityWorld>>,
    prediction: Option<Res<crate::ClientPredictionState>>,
    descriptor: Option<Res<crate::MatchDescriptor>>,
    bridge: Option<Res<MasterBridge>>,
    udp_hub: Option<Res<UdpAuthorityHub>>,
    udp_link: Option<Res<UdpClientLink>>,
    mut commands: Commands,
) {
    if let Some(bridge) = bridge {
        let Some(descriptor) = descriptor.as_deref() else {
            return;
        };
        let world = authority
            .as_ref()
            .map(|authority| &authority.0)
            .or_else(|| prediction.as_ref().map(|prediction| prediction.0.world()));
        if world.is_none_or(|world| !world.has_world_clip()) {
            return;
        }
        let Some(mut hello) = handshake_for_match(world, Some(descriptor)) else {
            return;
        };
        hello.limits.max_packet_bytes = RELAY_PACKET_BYTES as u32;
        if udp_hub.is_none() && matches!(bridge.state(), MasterBridgeState::Hosting { .. }) {
            let mut hub = UdpAuthorityHub::relay(hello, 1, bridge.mailbox());
            hub.attach_bootstrap(Arc::clone(&bridge.bootstrap));
            commands.insert_resource(hub);
        }
        if udp_link.is_none()
            && matches!(
                bridge.state(),
                MasterBridgeState::Joining { .. } | MasterBridgeState::Joined { .. }
            )
        {
            let mut link = UdpClientLink::relay(hello, bridge.mailbox());
            link.attach_bootstrap(Arc::clone(&bridge.bootstrap));
            diag::info!(
                Net,
                "master client relay mailbox — protocol={} offering map={:016x} weapons={:016x} classes={:016x}",
                crate::PROTOCOL_VERSION,
                hello.content.map,
                hello.content.weapons,
                hello.content.classes
            );
            commands.insert_resource(link);
        }
        return;
    }
    match &intent.0 {
        MasterLaunchMode::Disabled | MasterLaunchMode::Browser(_) => {}
        MasterLaunchMode::Host(config) => {
            if *role != crate::RuntimeRole::Listen || udp_hub.is_some() {
                return;
            }
            let relay = spawn_host(config.clone(), settings.player_name.clone());
            diag::info!(
                Net,
                "master public lobby arming for {}",
                config.target.address
            );
            commands.insert_resource(relay);
        }
        MasterLaunchMode::Join(config) => {
            if *role != crate::RuntimeRole::Client || udp_link.is_some() {
                return;
            }
            let relay = spawn_join(config.clone(), settings.player_name.clone());
            diag::info!(
                Net,
                "master joining lobby advert {} through {} ({}/{})",
                config.advert_id,
                config.target.address,
                config.map,
                config.mode
            );
            commands.insert_resource(relay);
        }
    }
}

fn arm_master_browser(
    intent: Res<MasterLaunchIntent>,
    browser: Option<Res<MasterBrowser>>,
    mut commands: Commands,
) {
    if browser.is_some() {
        return;
    }
    let MasterLaunchMode::Browser(browser_config) = &intent.0 else {
        return;
    };
    let state = Arc::new(Mutex::new(MasterBrowserSnapshot {
        loading: true,
        have: browser_config.have,
        ..default()
    }));
    let refresh = Arc::new(AtomicU64::new(0));
    let cancel = CancellationToken::new();
    let worker_state = Arc::clone(&state);
    let worker_refresh = Arc::clone(&refresh);
    let worker_cancel = cancel.clone();
    let target = browser_config.target.clone();

    let worker = std::thread::Builder::new()
        .name("iw4l-master-browser".into())
        .spawn(move || browser_worker(target, worker_state, worker_refresh, worker_cancel))
        .expect("spawn master browser thread");
    commands.insert_resource(MasterBrowser {
        state,
        refresh,
        cancel,
        worker: Some(worker),
    });
}

fn apply_master_menu_action(
    mut pending: ResMut<PendingMasterMenuAction>,
    mut intent: ResMut<MasterLaunchIntent>,
    mut role: ResMut<crate::RuntimeRole>,
    browser: Option<Res<MasterBrowser>>,
    bridge: Option<Res<MasterBridge>>,
    mut commands: Commands,
) {
    let Some(action) = pending.0.take() else {
        return;
    };
    match action {
        MasterMenuAction::Refresh => {
            if let Some(browser) = browser {
                browser.refresh();
            }
            return;
        }
        MasterMenuAction::UpdateLobby { map, mode } => {
            if let Some(bridge) = bridge {
                bridge.send(MasterBridgeCommand::UpdateLobby { map, mode });
            } else {
                diag::warn!(Net, "master lobby update requested without a live bridge");
            }
            return;
        }
        MasterMenuAction::StartMatch { map, mode } => {
            if let Some(bridge) = bridge {
                bridge.send(MasterBridgeCommand::StartMatch { map, mode });
            } else {
                diag::warn!(Net, "master lobby start requested without a live bridge");
            }
            return;
        }
        MasterMenuAction::VoteToSkip => {
            if let Some(bridge) = bridge {
                bridge.send(MasterBridgeCommand::VoteToSkip);
            } else {
                diag::warn!(Net, "master lobby vote requested without a live bridge");
            }
            return;
        }
        MasterMenuAction::LeaveLobby => {
            if let Some(bridge) = bridge {
                bridge.leave();
                commands.remove_resource::<MasterBridge>();
            }
            commands.remove_resource::<UdpAuthorityHub>();
            commands.remove_resource::<UdpClientLink>();
            let browser = match &intent.0 {
                MasterLaunchMode::Browser(config) => Some(config.clone()),
                MasterLaunchMode::Host(config) => Some(BrowserConfig {
                    target: config.target.clone(),
                    have: config.have,
                }),
                MasterLaunchMode::Join(config) => Some(BrowserConfig {
                    target: config.target.clone(),
                    have: config.have,
                }),
                MasterLaunchMode::Disabled => None,
            };
            if let Some(browser) = browser {
                intent.0 = MasterLaunchMode::Browser(browser);
            }
            *role = crate::RuntimeRole::Listen;
            return;
        }
        MasterMenuAction::Host { .. } | MasterMenuAction::Join { .. } => {}
    }
    let MasterLaunchMode::Browser(browser) = &intent.0 else {
        return;
    };
    let browser = browser.clone();
    match action {
        MasterMenuAction::Refresh
        | MasterMenuAction::UpdateLobby { .. }
        | MasterMenuAction::StartMatch { .. }
        | MasterMenuAction::VoteToSkip
        | MasterMenuAction::LeaveLobby => unreachable!(),
        MasterMenuAction::Host { map, mode } => {
            let requires = match content_required_by_map(&map) {
                Ok(value) => value,
                Err(error) => {
                    diag::warn!(Net, "master host content: {error}");
                    return;
                }
            };
            intent.0 = MasterLaunchMode::Host(HostConfig {
                auto_start_map: false,
                target: browser.target,
                name: std::env::var("IW4L_MASTER_HOST_NAME").unwrap_or_else(|_| "iw4l host".into()),
                map,
                mode,
                max_players: match parse_max_players() {
                    Ok(value) => value,
                    Err(error) => {
                        diag::warn!(Net, "master host settings: {error}");
                        return;
                    }
                },
                requires,
                have: browser.have,
            });
            *role = crate::RuntimeRole::Listen;
        }
        MasterMenuAction::Join {
            advert_id,
            map,
            mode,
        } => {
            intent.0 = MasterLaunchMode::Join(JoinConfig {
                target: browser.target,
                advert_id,
                map,
                mode,
                have: browser.have,
            });
            *role = crate::RuntimeRole::Client;
        }
    }
}

static BRIDGE_INCARNATION: AtomicU64 = AtomicU64::new(1);
static ATTEMPT_ID: AtomicU64 = AtomicU64::new(1);

fn apply_master_lifecycle(
    bridge: Option<Res<MasterBridge>>,
    mut hub: Option<ResMut<UdpAuthorityHub>>,
    mut authority: Option<ResMut<AuthorityWorld>>,
    mut pending_notify: Option<ResMut<crate::PendingGameNotify>>,
) {
    let Some(bridge) = bridge else {
        return;
    };
    let state = bridge.state();
    if let Some(hub) = hub.as_mut()
        && matches!(state, MasterBridgeState::Hosting { .. })
    {
        hub.reconcile_relay_membership(state.members(), state.identity().member_id);
    }
    if let Some(hub) = hub.as_mut() {
        for admission in hub.take_committed_admissions() {
            let client = hub.client_of_member(admission.member_id);
            bridge.admit_enter(
                admission.member_id,
                admission.epoch,
                admission.bootstrap_id,
                Some(admission.connection_id),
                client.map(|client| client.0).unwrap_or(0),
            );

            if admission.first_commit
                && let Some(client) = client
                && let Some(authority) = authority.as_ref()
                && let Some(pending) = pending_notify.as_mut()
            {
                pending.push_connected(crate::client_name_string(&authority.0, client));
            }
        }
    }
    for fact in bridge.drain_facts() {
        match fact {
            MasterLifecycleFact::MemberLeft { member_id } => {
                let name = hub.as_ref().and_then(|hub| {
                    let client = hub.client_of_member(member_id)?;
                    let authority = authority.as_ref()?;
                    Some(crate::client_name_string(&authority.0, client))
                });
                if let Some(hub) = hub.as_mut()
                    && let Some(client) = hub.retire_member(member_id)
                    && let Some(authority) = authority.as_mut()
                {
                    authority.0.retire_client(client);
                }
                if let (Some(name), Some(pending)) = (name, pending_notify.as_mut()) {
                    pending.push_left(name);
                }
            }
            MasterLifecycleFact::AdmissionFailed { member_id, reason } => {
                diag::warn!(Net, "admission failed for member {member_id}: {reason}");
                if let Some(hub) = hub.as_mut()
                    && let Some(client) = hub.deny_member(member_id)
                    && let Some(authority) = authority.as_mut()
                {
                    authority.0.retire_client(client);
                }
            }
            MasterLifecycleFact::SessionClosed { .. } => {}
        }
    }
}

pub fn register_master_bridge(app: &mut App) {
    app.init_resource::<PendingMasterMenuAction>()
        .add_systems(
            Update,
            (arm_master_browser, apply_master_menu_action)
                .chain()
                .in_set(crate::ClientSet::Load),
        )
        .add_systems(
            Update,
            (
                arm_master_bridge,
                open_hosted_epoch_when_world_is_live,
                apply_master_lifecycle,
            )
                .chain()
                .in_set(crate::ClientSet::Load),
        )
        .add_systems(
            Update,
            crate::signon::drive_match_boundary
                .in_set(crate::ClientSet::Load)
                .after(frame::SessionSwapApplied),
        )
        .add_systems(Update, observe_master_bridge.in_set(crate::ClientSet::Diag));
    {
        app.add_systems(
            FixedUpdate,
            refresh_relay_authority_hello.in_set(crate::AuthoritySet::Advance),
        );
        app.add_systems(
            FixedUpdate,
            report_committed_authority_progress.after(crate::AuthoritySet::Snapshot),
        );
    }
}

fn report_committed_authority_progress(
    bridge: Option<Res<MasterBridge>>,
    server_tick: Option<Res<crate::authority::runtime::ServerTick>>,
    hold: Option<Res<crate::AuthorityLoadHold>>,
) {
    let Some(bridge) = bridge else {
        return;
    };
    if hold.is_some_and(|hold| hold.0) {
        return;
    }
    if server_tick.is_some_and(|tick| tick.0.is_some()) {
        bridge.report_authority_progress();
    }
}

fn open_hosted_epoch_when_world_is_live(
    bridge: Option<Res<MasterBridge>>,
    intent: Res<MasterLaunchIntent>,
    admission: Res<crate::ClientAdmission>,
    has_world: Option<Res<frame::HasWorld>>,
    launch: Option<Res<frame::LaunchIdentity>>,
    hold: Option<Res<crate::AuthorityLoadHold>>,
    mut sent: Local<Option<(u64, frame::LocalLoadKey)>>,
) {
    let Some(bridge) = bridge else {
        return;
    };
    let MasterLaunchMode::Host(config) = &intent.0 else {
        return;
    };
    if !config.auto_start_map
        || !has_world.is_some_and(|world| world.0)
        || hold.is_some_and(|hold| hold.0)
    {
        return;
    }

    let Some(load) = admission
        .core
        .installed()
        .filter(|load| load.match_key.is_none())
    else {
        return;
    };
    let key = (bridge.incarnation(), load);
    if *sent == Some(key) {
        return;
    }
    let MasterBridgeState::Hosting {
        in_match: false, ..
    } = bridge.state()
    else {
        return;
    };
    let Some(launch) = launch else {
        return;
    };
    bridge.start_hosted_match(launch.zone.clone(), config.mode.clone());
    *sent = Some(key);
}

fn observe_master_bridge(
    bridge: Option<Res<MasterBridge>>,
    mut previous: Local<Option<MasterBridgeState>>,
    mut seen_start: Local<Option<(AdvertId, u32)>>,
    mut pending_start: ResMut<MasterMatchStart>,
) {
    let Some(bridge) = bridge else {
        return;
    };
    let current = bridge.state();
    let identity = current.identity();
    if current.in_match()
        && observe_live_start(true, identity.epoch)
        && seen_start.as_ref() != Some(&(identity.room_id, identity.epoch))
        && let (Some(map), Some(mode)) = (current.map(), current.mode())
    {
        pending_start.0 = Some(MasterMatchOffer {
            map: map.to_owned(),
            mode: mode.to_owned(),
            match_key: identity.match_key(),
        });
        *seen_start = Some((identity.room_id, identity.epoch));
    }
    if previous.as_ref() == Some(&current) {
        return;
    }
    match &current {
        MasterBridgeState::Failed { error, identity } => {
            diag::warn!(
                Net,
                "master relay failed attempt={} room={} epoch={}: {error}",
                identity.attempt_id,
                identity.room_id,
                identity.epoch
            );
        }
        _ => diag::info!(Net, "master relay: {current:?}"),
    }
    *previous = Some(current);
}

fn host_transition_event(line: &str) -> &str {
    line.split(" event=")
        .nth(1)
        .and_then(|rest| rest.split(" stage_after=").next())
        .unwrap_or("")
}

fn should_log_host_transition(applied: &HostMatchApply) -> bool {
    if !applied.effects.is_empty() {
        return true;
    }
    let event = host_transition_event(&applied.transition);
    event != "tick" && event != "authority progress"
}
