use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::asset_graph::{
    AssetEdge, AssetEdgeCensus, AssetEdgeReason, FpvMeshSpace, FxSpace, MaterialSpace,
    ProjectileModelSpace, TracerSpace, WorldWeaponSpace, XAnimSpace, ZoneOwner,
};
use asset_iw4::size::{WEAPON_ANIM_COUNT, weap_anim};

use fastfile_iw4::{
    Ptr, ScriptStrings, WeaponIdleCapture, WeaponMovementOfsCapture, ZonePtr, ZoneStream,
};
use weapon_iw4::{WEAPON_ANIM_SLOTS, weap_anim_extra};
use weapon_iw4::{WeaponIdleInputs, WeaponMovementOfsInputs};

mod iw5;
mod registry;
mod t5;

pub use registry::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponBodyFacts {
    pub body_resolved: bool,
    pub fire_time_ms: i32,
    pub impact_type: i32,
    pub raise_time_ms: i32,
    pub drop_time_ms: i32,
    pub fire_delay_ms: i32,
    pub hold_fire_time_ms: i32,
    pub weap_type: i32,
    pub player_anim_type: i32,
    pub weap_class: i32,
    pub offhand_class: i32,
    pub shots_per_fire: i32,
    pub ammo_index: i32,
    pub clip_index: i32,
    pub ammo_counter_clip: i32,
    pub low_ammo_warning_threshold: f32,
    pub hip_spread_stand_min: f32,
    pub hip_spread_ducked_min: f32,
    pub hip_spread_prone_min: f32,
    pub hip_spread_stand_max: f32,
    pub hip_spread_ducked_max: f32,
    pub hip_spread_prone_max: f32,
    pub hip_spread_decay_rate: f32,
    pub hip_spread_fire_add: f32,
    pub hip_spread_turn_add: f32,
    pub hip_spread_move_add: f32,
    pub hip_spread_ducked_decay: f32,
    pub hip_spread_prone_decay: f32,
    pub i_reticle_side_size: i32,
    pub i_reticle_min_ofs: i32,
    pub hip_reticle_side_pos: f32,
    pub ads_aim_pitch: f32,
    pub ads_crosshair_in_frac: f32,
    pub ads_crosshair_out_frac: f32,
    pub ads_spread: f32,
    pub aim_down_sight: bool,
    pub ads_zoom_fov: f32,
    pub ads_dof: Option<[f32; 2]>,
    pub ads_zoom_in_frac: f32,
    pub ads_zoom_out_frac: f32,
    pub no_ads_when_mag_empty: bool,
    pub inherits_perks: bool,
    pub ads_in_rate: f32,
    pub ads_out_rate: f32,
    pub rechamber_while_ads: bool,
    pub ads_fire_only: bool,
    pub melee_damage: i32,
    pub overlay_reticle: i32,
    pub overlay_interface: i32,
    pub ads_overlay_width: f32,
    pub ads_overlay_height: f32,
    pub melee_time_ms: i32,
    pub melee_delay_ms: i32,
    pub melee_charge_time_ms: i32,
    pub melee_charge_delay_ms: i32,
    pub knife_model: u32,
    pub quick_raise_time_ms: i32,
    pub quick_drop_time_ms: i32,
    pub select_requires_ammo_at_0x667: Option<bool>,
    pub offhand_hold_is_cancelable_at_0x681: Option<bool>,
    pub move_speed_scale: f32,
    pub ads_move_speed_scale: f32,
    pub sprint_duration_scale: f32,
    pub stance_ofs_at_0x168: [f32; 3],
    pub stance_ofs_at_0x18c: [f32; 3],
    pub night_vision_wear_time: i32,
    pub ads_bob_factor_at_0x330: f32,
    pub ads_view_bob_mult_at_0x334: f32,
    pub movement: WeaponMovementOfsInputs,
    pub idle: WeaponIdleInputs,
    pub clip_size: i32,
    pub penetrate_type: i32,
    pub penetrate_multiplier: f32,
    pub motion_tracker: bool,
    pub rifle_bullet: bool,
    pub inventory_type: i32,
    pub fire_type: i32,
    pub max_ammo: i32,
    pub damage: i32,
    pub rechamber_time_ms: i32,
    pub rechamber_bolt_time_ms: i32,
    pub rechamber_bolt_delay_ms: i32,
    pub reload_time_ms: i32,
    pub reload_show_rocket_time_ms: i32,
    pub reload_empty_time_ms: i32,
    pub reload_add_time_ms: i32,
    pub reload_empty_add_time_ms: i32,
    pub reload_start_time_ms: i32,
    pub reload_start_add_time_ms: i32,
    pub reload_end_time_ms: i32,
    pub dual_mag: Option<weapon_iw4::DualMagTimes>,
    pub kill_icon_ratio: i32,
    pub flip_kill_icon: bool,
    pub reload_ammo_add: i32,
    pub reload_start_add: i32,
    pub no_partial_reload: bool,
    pub bolt_action: bool,
    pub segmented_reload: bool,
    pub sprint_raise_time_ms: i32,
    pub sprint_loop_time_ms: i32,
    pub sprint_drop_time_ms: i32,
    pub fuse_time_ms: i32,
    pub cook_off_hold: bool,
    pub clip_only: bool,
    pub timed_detonation: bool,
    pub proj_impact_explode: bool,
    pub stick_to_players: bool,
    pub explosion_radius: i32,
    pub explosion_radius_min: i32,
    pub explosion_inner_damage: i32,
    pub explosion_outer_damage: i32,
    pub projectile_speed: i32,
    pub projectile_speed_up: i32,
    pub projectile_speed_forward: i32,
    pub projectile_activate_dist: i32,
    pub projectile_explosion_type: i32,
    pub parallel_bounce: Option<[f32; 31]>,
    pub perpendicular_bounce: Option<[f32; 31]>,
    pub location_damage_mult: Option<[f32; 20]>,
    pub start_ammo: i32,
    pub ammo_count_clip_relative: bool,
    pub min_damage: i32,
    pub min_player_damage: i32,
    pub max_damage_range: f32,
    pub min_damage_range: f32,
    pub kick: WeaponKickFacts,
    pub sway: WeaponSwayFacts,
    pub dual_wield_view_model_offset: f32,
    pub no_dual_wield: bool,
}

impl WeaponBodyFacts {
    pub fn start_ammo_rounds(&self) -> i32 {
        leftover_clip_relative_rounds(
            self.start_ammo,
            self.clip_size,
            self.ammo_count_clip_relative,
        )
    }

    pub fn max_ammo_rounds(&self) -> i32 {
        leftover_clip_relative_rounds(self.max_ammo, self.clip_size, self.ammo_count_clip_relative)
    }
}

fn leftover_clip_relative_rounds(count: i32, clip_size: i32, clip_relative: bool) -> i32 {
    if clip_relative && clip_size > 0 && count > 0 {
        count.saturating_mul(clip_size)
    } else {
        count
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponKickFacts {
    pub f_ads_view_kick_center_speed: f32,
    pub f_hip_view_kick_center_speed: f32,
    pub gun_max_pitch: f32,
    pub gun_max_yaw: f32,
    pub ads_gun_kick_reduced_kick_bullets: i32,
    pub ads_gun_kick_reduced_kick_percent: f32,
    pub ads_gun_kick_pitch_min: f32,
    pub ads_gun_kick_pitch_max: f32,
    pub ads_gun_kick_yaw_min: f32,
    pub ads_gun_kick_yaw_max: f32,
    pub ads_gun_kick_accel: f32,
    pub ads_gun_kick_speed_max: f32,
    pub ads_gun_kick_speed_decay: f32,
    pub ads_gun_kick_static_decay: f32,
    pub ads_view_kick_pitch_min: f32,
    pub ads_view_kick_pitch_max: f32,
    pub ads_view_kick_yaw_min: f32,
    pub ads_view_kick_yaw_max: f32,
    pub hip_gun_kick_reduced_kick_bullets: i32,
    pub hip_gun_kick_reduced_kick_percent: f32,
    pub hip_gun_kick_pitch_min: f32,
    pub hip_gun_kick_pitch_max: f32,
    pub hip_gun_kick_yaw_min: f32,
    pub hip_gun_kick_yaw_max: f32,
    pub hip_gun_kick_accel: f32,
    pub hip_gun_kick_speed_max: f32,
    pub hip_gun_kick_speed_decay: f32,
    pub hip_gun_kick_static_decay: f32,
    pub hip_view_kick_pitch_min: f32,
    pub hip_view_kick_pitch_max: f32,
    pub hip_view_kick_yaw_min: f32,
    pub hip_view_kick_yaw_max: f32,
}

impl WeaponKickFacts {
    fn from_capture(c: fastfile_iw4::WeaponKickCapture) -> Self {
        Self {
            f_ads_view_kick_center_speed: c.f_ads_view_kick_center_speed,
            f_hip_view_kick_center_speed: c.f_hip_view_kick_center_speed,
            gun_max_pitch: c.gun_max_pitch,
            gun_max_yaw: c.gun_max_yaw,
            ads_gun_kick_reduced_kick_bullets: c.ads_gun_kick_reduced_kick_bullets,
            ads_gun_kick_reduced_kick_percent: c.ads_gun_kick_reduced_kick_percent,
            ads_gun_kick_pitch_min: c.ads_gun_kick_pitch_min,
            ads_gun_kick_pitch_max: c.ads_gun_kick_pitch_max,
            ads_gun_kick_yaw_min: c.ads_gun_kick_yaw_min,
            ads_gun_kick_yaw_max: c.ads_gun_kick_yaw_max,
            ads_gun_kick_accel: c.ads_gun_kick_accel,
            ads_gun_kick_speed_max: c.ads_gun_kick_speed_max,
            ads_gun_kick_speed_decay: c.ads_gun_kick_speed_decay,
            ads_gun_kick_static_decay: c.ads_gun_kick_static_decay,
            ads_view_kick_pitch_min: c.ads_view_kick_pitch_min,
            ads_view_kick_pitch_max: c.ads_view_kick_pitch_max,
            ads_view_kick_yaw_min: c.ads_view_kick_yaw_min,
            ads_view_kick_yaw_max: c.ads_view_kick_yaw_max,
            hip_gun_kick_reduced_kick_bullets: c.hip_gun_kick_reduced_kick_bullets,
            hip_gun_kick_reduced_kick_percent: c.hip_gun_kick_reduced_kick_percent,
            hip_gun_kick_pitch_min: c.hip_gun_kick_pitch_min,
            hip_gun_kick_pitch_max: c.hip_gun_kick_pitch_max,
            hip_gun_kick_yaw_min: c.hip_gun_kick_yaw_min,
            hip_gun_kick_yaw_max: c.hip_gun_kick_yaw_max,
            hip_gun_kick_accel: c.hip_gun_kick_accel,
            hip_gun_kick_speed_max: c.hip_gun_kick_speed_max,
            hip_gun_kick_speed_decay: c.hip_gun_kick_speed_decay,
            hip_gun_kick_static_decay: c.hip_gun_kick_static_decay,
            hip_view_kick_pitch_min: c.hip_view_kick_pitch_min,
            hip_view_kick_pitch_max: c.hip_view_kick_pitch_max,
            hip_view_kick_yaw_min: c.hip_view_kick_yaw_min,
            hip_view_kick_yaw_max: c.hip_view_kick_yaw_max,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponSwayFacts {
    pub sway_max_angle: f32,
    pub sway_lerp_speed: f32,
    pub sway_pitch_scale: f32,
    pub sway_yaw_scale: f32,
    pub sway_horiz_scale: f32,
    pub sway_vert_scale: f32,
    pub sway_shell_shock_scale: f32,
    pub ads_sway_max_angle: f32,
    pub ads_sway_lerp_speed: f32,
    pub ads_sway_pitch_scale: f32,
    pub ads_sway_yaw_scale: f32,
    pub ads_sway_horiz_scale: f32,
    pub ads_sway_vert_scale: f32,
}

impl WeaponSwayFacts {
    fn from_capture(c: fastfile_iw4::WeaponSwayCapture) -> Self {
        Self {
            sway_max_angle: c.sway_max_angle,
            sway_lerp_speed: c.sway_lerp_speed,
            sway_pitch_scale: c.sway_pitch_scale,
            sway_yaw_scale: c.sway_yaw_scale,
            sway_horiz_scale: c.sway_horiz_scale,
            sway_vert_scale: c.sway_vert_scale,
            sway_shell_shock_scale: c.sway_shell_shock_scale,
            ads_sway_max_angle: c.ads_sway_max_angle,
            ads_sway_lerp_speed: c.ads_sway_lerp_speed,
            ads_sway_pitch_scale: c.ads_sway_pitch_scale,
            ads_sway_yaw_scale: c.ads_sway_yaw_scale,
            ads_sway_horiz_scale: c.ads_sway_horiz_scale,
            ads_sway_vert_scale: c.ads_sway_vert_scale,
        }
    }

    pub fn hip_params(&self) -> weapon_iw4::WeaponSwayParams {
        weapon_iw4::WeaponSwayParams {
            max_angle: self.sway_max_angle,
            lerp_speed: self.sway_lerp_speed,
            pitch_scale: self.sway_pitch_scale,
            yaw_scale: self.sway_yaw_scale,
            horiz_scale: self.sway_horiz_scale,
            vert_scale: self.sway_vert_scale,
        }
    }

    pub fn ads_params(&self) -> weapon_iw4::WeaponSwayParams {
        weapon_iw4::WeaponSwayParams {
            max_angle: self.ads_sway_max_angle,
            lerp_speed: self.ads_sway_lerp_speed,
            pitch_scale: self.ads_sway_pitch_scale,
            yaw_scale: self.ads_sway_yaw_scale,
            horiz_scale: self.ads_sway_horiz_scale,
            vert_scale: self.ads_sway_vert_scale,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacOffhandBucket {
    Lethal,
    Tactical,
}

pub fn cac_offhand_bucket(offhand_class: i32) -> Option<CacOffhandBucket> {
    match offhand_class {
        1 | 4 | 5 => Some(CacOffhandBucket::Lethal),
        2 | 3 => Some(CacOffhandBucket::Tactical),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct CatalogWeapon {
    pub name: String,
    pub weap_def: Option<(u8, u32)>,
    pub display_name_key: Option<String>,
    pub reticle: WeaponReticleAssets,
    pub hud_material_edges: WeaponHudMaterialEdges,
    pub overlay_material: Option<String>,
    pub overlay_image: Option<String>,
    pub reticle_center_slot: Option<Ptr>,
    pub reticle_side_slot: Option<Ptr>,
    pub overlay_material_slot: Option<Ptr>,
    pub scope_name: Option<String>,
    pub scope_rows: [Iw5ScopeRow; 6],
    pub iw5_attachment_slots: [Option<String>; fastfile_iw5::size::WEAPON_ATTACHMENT_SLOT_COUNT],
    pub iw5_reload_overrides: Vec<fastfile_iw5::ReloadOverride>,
    pub iw5_anim_overrides: Vec<LeftoverAnimOverride>,
    pub iw5_fx_overrides: Vec<Iw5FxOverride>,
    pub iw5_notetrack_overrides: Vec<Iw5NotetrackOverride>,
    pub hud_icon: Option<String>,
    pub hud_icon_slot: Option<Ptr>,
    pub pickup_icon: Option<String>,
    pub pickup_icon_slot: Option<Ptr>,
    pub pickup_icon_image: Option<String>,
    pub pickup_icon_ratio: i32,
    pub hud_icon_ratio: i32,
    pub hud_icon_image: Option<String>,
    pub dpad_icon: Option<String>,
    pub dpad_icon_image: Option<String>,
    pub dpad_icon_atlas: Option<[u8; 2]>,
    pub dpad_icon_ratio: i32,
    pub kill_icon: Option<String>,
    pub kill_icon_slot: Option<Ptr>,
    pub kill_icon_image: Option<String>,
    pub proj_trail: Option<String>,
    pub proj_trail_slot: Option<Ptr>,
    pub proj_beacon: Option<String>,
    pub proj_beacon_slot: Option<Ptr>,
    pub proj_ignition: Option<String>,
    pub proj_ignition_slot: Option<Ptr>,
    pub projectile_fx: WeaponProjectileFx,
    pub gun_xmodel: Option<String>,
    pub hand_xmodel: Option<String>,
    pub world_model: Option<String>,
    pub projectile_model: Option<String>,
    pub rocket_model: Option<String>,
    pub sz_xanims: [Option<String>; WEAPON_ANIM_SLOTS],
    pub sz_xanims_right: [Option<String>; WEAPON_ANIM_SLOTS],
    pub sz_xanims_left: [Option<String>; WEAPON_ANIM_SLOTS],
    pub hide_tags: Vec<String>,
    pub sounds: WeaponSoundAliases,
    pub combat_fx: WeaponCombatFx,
    pub(crate) combat_slots: CombatFxSlots,
    pub facts: WeaponBodyFacts,
}

#[derive(Clone, Debug, Default)]
pub struct Iw5ScopeRow {
    pub scope: Option<String>,
    pub display_name: Option<String>,
    pub attachment_type: i32,
    pub weapon_type: i32,
    pub weapon_class: i32,
    pub load_index: i32,
    pub overlay: Option<String>,
    pub overlay_lowres: Option<String>,
    pub overlay_emp: Option<String>,
    pub overlay_emp_lowres: Option<String>,
    pub view_model: Option<String>,
    pub world_model: Option<String>,
    pub view_models: [Option<String>; fastfile_iw5::size::ATTACH_MODEL_COUNT],
    pub world_models: [Option<String>; fastfile_iw5::size::ATTACH_MODEL_COUNT],
    pub reticle_models: [Option<String>; fastfile_iw5::size::ATTACH_RETICLE_COUNT],
    pub thermal: bool,
    pub width: f32,
    pub height: f32,
    pub ads_zoom_fov: f32,
    pub ads_zoom_in_frac: f32,
    pub ads_zoom_out_frac: f32,
    pub sight: Option<fastfile_iw5::AttachmentSight>,
    pub ammo_general: Option<fastfile_iw5::AttachmentAmmoGeneral>,
    pub reload: Option<fastfile_iw5::AttachmentReload>,
    pub add_ons: Option<fastfile_iw5::AttachmentAddOns>,
    pub general: Option<fastfile_iw5::AttachmentGeneral>,
    pub aim_assist: Option<fastfile_iw5::AttachmentAimAssist>,
    pub ammunition: Option<fastfile_iw5::AttachmentAmmunition>,
    pub damage: Option<fastfile_iw5::AttachmentDamage>,
    pub location_damage: Option<[f32; 19]>,
    pub idle_settings: Option<fastfile_iw5::AttachmentIdleSettings>,
    pub ads_settings: Option<fastfile_iw5::AttachmentAdsSettings>,
    pub ads_settings_main: Option<fastfile_iw5::AttachmentAdsSettings>,
    pub hip_spread: Option<fastfile_iw5::AttachmentHipSpread>,
    pub gun_kick: Option<fastfile_iw5::AttachmentGunKick>,
    pub view_kick: Option<[f32; 10]>,
    pub scales: fastfile_iw5::AttachmentScales,
    pub hide_iron_sights: bool,
    pub share_ammo_with_alt: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Iw5AttachmentSelection {
    pub scope: u8,
    pub underbarrel: u8,
    pub others: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Iw5ConfigurationCandidate {
    pub selection: crate::WeaponSelection,
    pub base_id: u32,
    pub native: Result<Iw5AttachmentSelection, crate::ConfigurationRefusal>,
    pub primary_assets: Vec<String>,
    pub primary_ads_zoom_fov: Option<f32>,
    pub primary_ads_aim_pitch: Option<f32>,
}

impl Iw5AttachmentSelection {
    pub fn fields(self) -> u16 {
        u16::from(self.scope) | (u16::from(self.underbarrel) << 3) | (u16::from(self.others) << 5)
    }

    fn override_candidates(self) -> [u16; 3] {
        let mut candidates = [0; 3];
        let mut count = 0;
        let mut push = |condition| {
            candidates[count.min(2)] = condition;
            count += 1;
        };
        if (1..=6).contains(&self.scope) {
            push(u16::from(self.scope));
        }
        if (1..=3).contains(&self.underbarrel) {
            push(u16::from(self.underbarrel) << 3);
        }
        for bit in 0..4 {
            if self.others & (1 << bit) != 0 {
                push(1 << (5 + bit));
            }
        }
        candidates
    }

    pub fn contains_condition(self, condition: u16) -> bool {
        if condition == 0 {
            return true;
        }
        self.override_candidates().contains(&condition)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponReticleAssets {
    pub center_material: Option<String>,
    pub side_material: Option<String>,
    pub center_edge: AssetEdge<MaterialSpace>,
    pub side_edge: AssetEdge<MaterialSpace>,
    pub center_image: Option<String>,
    pub side_image: Option<String>,
    pub center_size: i32,
    pub side_size: i32,

    /// Whether the zone authored a reticle material at all. The slot pointers
    /// that answered this during the walk stay on the build row: the HUD, and
    /// the edge stamping beside it, only ever asked whether there was one.
    pub center_authored: bool,
    pub side_authored: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeaponHudMaterialEdges {
    pub overlay: AssetEdge<MaterialSpace>,
    pub hud_icon: AssetEdge<MaterialSpace>,
    pub pickup_icon: AssetEdge<MaterialSpace>,
    pub kill_icon: AssetEdge<MaterialSpace>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeaponProjectileFx {
    pub trail: AssetEdge<FxSpace>,
    pub beacon: AssetEdge<FxSpace>,
    pub ignition: AssetEdge<FxSpace>,
}

impl WeaponProjectileFx {
    pub fn edges(self) -> [AssetEdge<FxSpace>; 3] {
        [self.trail, self.beacon, self.ignition]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotetrackConvention {
    #[default]
    SoundMap,
    InlinePrefix,
}

impl NotetrackConvention {
    pub const fn dump_token(self) -> &'static str {
        match self {
            Self::SoundMap => "sound_map",
            Self::InlinePrefix => "inline_prefix",
        }
    }
}

pub const T5_NOTE_SOUND_PREFIX: &str = "sndnt#";

pub const T5_NOTE_RUMBLE_PREFIX: &str = "rmbnt#";

pub fn t5_inline_note_alias<'a>(note: &'a str, prefix: &str) -> Option<&'a str> {
    let (head, tail) = note.split_at_checked(prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then_some(tail)
        .filter(|alias| !alias.is_empty())
}

#[derive(Clone, Debug, Default)]
pub struct LinkedNotetrackAction {
    pub sound_alias: Option<String>,
    pub rumble_alias: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeaponDependencyGap {
    pub id: u32,
    pub kind: &'static str,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponSoundAliases {
    pub fire: Option<String>,
    pub fire_player: Option<String>,
    pub empty_fire: Option<String>,
    pub empty_fire_player: Option<String>,
    pub melee_swipe: Option<String>,
    pub melee_swipe_player: Option<String>,
    pub melee_hit: Option<String>,
    pub melee_miss: Option<String>,
    pub pickup: Option<String>,
    pub pickup_player: Option<String>,
    pub ammo_pickup: Option<String>,
    pub ammo_pickup_player: Option<String>,
    pub pullback: Option<String>,
    pub pullback_player: Option<String>,
    pub reload: Option<String>,
    pub reload_player: Option<String>,
    pub reload_empty: Option<String>,
    pub reload_empty_player: Option<String>,
    pub reload_start: Option<String>,
    pub reload_start_player: Option<String>,
    pub reload_end: Option<String>,
    pub reload_end_player: Option<String>,
    pub rechamber: Option<String>,
    pub rechamber_player: Option<String>,
    pub alt_switch: Option<String>,
    pub alt_switch_player: Option<String>,
    pub raise: Option<String>,
    pub raise_player: Option<String>,
    pub first_raise: Option<String>,
    pub first_raise_player: Option<String>,
    pub putaway: Option<String>,
    pub putaway_player: Option<String>,
    pub proj_explosion: Option<String>,
    pub projectile: Option<String>,
    pub proj_ignition_sound: Option<String>,
    pub bounce: [Option<String>; asset_iw4::size::SURF_TYPE_NUM],
    pub notetrack_sound_map: Vec<(String, String)>,
    pub notetrack_rumble_map: Vec<(String, String)>,
    pub fire_player_akimbo: Option<String>,
    pub fire_loop: Option<String>,
    pub fire_loop_player: Option<String>,
    pub fire_stop: Option<String>,
    pub fire_stop_player: Option<String>,
    pub fire_last: Option<String>,
    pub fire_last_player: Option<String>,
    pub leftover_sound_overrides: Vec<LeftoverSoundOverride>,
    pub fire_ptr_kind: Option<&'static str>,
    pub fire_player_ptr_kind: Option<&'static str>,
    pub reload_player_ptr_kind: Option<&'static str>,
    pub notetrack_convention: NotetrackConvention,
}

macro_rules! weapon_sound_slots {
    ($($variant:ident => $field:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(usize)]
        pub enum WeaponSoundSlot {
            $($variant),+
        }

        impl WeaponSoundSlot {
            pub const ALL: [Self; weapon_sound_slots!(@count $($variant),+)] = [
                $(Self::$variant),+
            ];
        }

        impl WeaponSoundAliases {
            fn hint(&self, slot: WeaponSoundSlot) -> Option<&str> {
                match slot {
                    $(WeaponSoundSlot::$variant => self.$field.as_deref()),+
                }
            }
        }
    };
    (@count $($variant:ident),+) => {
        <[()]>::len(&[$(weapon_sound_slots!(@unit $variant)),+])
    };
    (@unit $variant:ident) => { () };
}

weapon_sound_slots! {
    Fire => fire,
    FirePlayer => fire_player,
    EmptyFire => empty_fire,
    EmptyFirePlayer => empty_fire_player,
    MeleeSwipe => melee_swipe,
    MeleeSwipePlayer => melee_swipe_player,
    MeleeHit => melee_hit,
    MeleeMiss => melee_miss,
    Pickup => pickup,
    PickupPlayer => pickup_player,
    AmmoPickup => ammo_pickup,
    AmmoPickupPlayer => ammo_pickup_player,
    Pullback => pullback,
    PullbackPlayer => pullback_player,
    Reload => reload,
    ReloadPlayer => reload_player,
    ReloadEmpty => reload_empty,
    ReloadEmptyPlayer => reload_empty_player,
    ReloadStart => reload_start,
    ReloadStartPlayer => reload_start_player,
    ReloadEnd => reload_end,
    ReloadEndPlayer => reload_end_player,
    Rechamber => rechamber,
    RechamberPlayer => rechamber_player,
    AltSwitch => alt_switch,
    AltSwitchPlayer => alt_switch_player,
    Raise => raise,
    RaisePlayer => raise_player,
    FirstRaise => first_raise,
    FirstRaisePlayer => first_raise_player,
    Putaway => putaway,
    PutawayPlayer => putaway_player,
    ProjectileExplosion => proj_explosion,
    Projectile => projectile,
    ProjIgnition => proj_ignition_sound,
    FireLast => fire_last,
    FireLastPlayer => fire_last_player,
}

impl WeaponSoundAliases {
    pub fn reachable_aliases(&self) -> Vec<&str> {
        let mut out = Vec::new();
        let slots = [
            self.fire.as_deref(),
            self.fire_player.as_deref(),
            self.empty_fire.as_deref(),
            self.empty_fire_player.as_deref(),
            self.melee_swipe.as_deref(),
            self.melee_swipe_player.as_deref(),
            self.melee_hit.as_deref(),
            self.melee_miss.as_deref(),
            self.pickup.as_deref(),
            self.pickup_player.as_deref(),
            self.ammo_pickup.as_deref(),
            self.ammo_pickup_player.as_deref(),
            self.pullback.as_deref(),
            self.pullback_player.as_deref(),
            self.reload.as_deref(),
            self.reload_player.as_deref(),
            self.reload_empty.as_deref(),
            self.reload_empty_player.as_deref(),
            self.reload_start.as_deref(),
            self.reload_start_player.as_deref(),
            self.reload_end.as_deref(),
            self.reload_end_player.as_deref(),
            self.rechamber.as_deref(),
            self.rechamber_player.as_deref(),
            self.alt_switch.as_deref(),
            self.alt_switch_player.as_deref(),
            self.raise.as_deref(),
            self.raise_player.as_deref(),
            self.first_raise.as_deref(),
            self.first_raise_player.as_deref(),
            self.putaway.as_deref(),
            self.putaway_player.as_deref(),
            self.proj_explosion.as_deref(),
            self.projectile.as_deref(),
            self.proj_ignition_sound.as_deref(),
            self.fire_player_akimbo.as_deref(),
            self.fire_loop.as_deref(),
            self.fire_loop_player.as_deref(),
            self.fire_stop.as_deref(),
            self.fire_stop_player.as_deref(),
            self.fire_last.as_deref(),
            self.fire_last_player.as_deref(),
        ];
        for slot in slots.into_iter().flatten() {
            if !slot.is_empty() {
                out.push(slot);
            }
        }
        for name in &self.bounce {
            if let Some(s) = name.as_deref().filter(|s| !s.is_empty()) {
                out.push(s);
            }
        }
        for (_, alias) in &self.notetrack_sound_map {
            if !alias.is_empty() {
                out.push(alias.as_str());
            }
        }
        for ov in &self.leftover_sound_overrides {
            if let Some(s) = ov.override_sound.as_deref().filter(|s| !s.is_empty()) {
                out.push(s);
            }
            if let Some(s) = ov.altmode_sound.as_deref().filter(|s| !s.is_empty()) {
                out.push(s);
            }
        }
        out
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LeftoverAnimOverride {
    pub attachment1: u16,
    pub attachment2: u16,
    pub anim_tree_type: u32,
    pub override_anim: Option<String>,
    pub altmode_anim: Option<String>,
    pub anim_time_ms: i32,
    pub alt_time_ms: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LeftoverSoundOverride {
    pub attachment1: u16,
    pub attachment2: u16,
    pub sound_type: u32,
    pub override_sound: Option<String>,
    pub altmode_sound: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Iw5FxOverride {
    pub attachment1: u16,
    pub attachment2: u16,
    pub fx_type: u32,
    pub override_fx: Option<String>,
    pub altmode_fx: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Iw5NotetrackOverride {
    pub attachment: u16,
    pub sound_map: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CombatFxSlots {
    view_flash: Option<Ptr>,
    world_flash: Option<Ptr>,
    view_shell_eject: Option<Ptr>,
    world_shell_eject: Option<Ptr>,
    view_last_shot_eject: Option<Ptr>,
    world_last_shot_eject: Option<Ptr>,
    explosion: Option<Ptr>,
    tracer: Option<Ptr>,
}

impl CombatFxSlots {
    fn last_shot_pair_authored(self) -> bool {
        self.view_last_shot_eject.is_some() && self.world_last_shot_eject.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeaponCombatFx {
    pub view_flash: AssetEdge<FxSpace>,
    pub view_flash_hint: Option<String>,
    pub world_flash: AssetEdge<FxSpace>,
    pub world_flash_hint: Option<String>,
    pub view_shell_eject: AssetEdge<FxSpace>,
    pub view_shell_eject_hint: Option<String>,
    pub world_shell_eject: AssetEdge<FxSpace>,
    pub world_shell_eject_hint: Option<String>,
    pub view_last_shot_eject: AssetEdge<FxSpace>,
    pub view_last_shot_eject_hint: Option<String>,
    pub world_last_shot_eject: AssetEdge<FxSpace>,
    pub world_last_shot_eject_hint: Option<String>,
    pub explosion: AssetEdge<FxSpace>,
    pub explosion_hint: Option<String>,
    pub tracer: AssetEdge<TracerSpace>,
    pub tracer_hint: Option<String>,
    last_shot_eject_pair_authored: bool,
}

fn present_bound<'a, S: crate::asset_graph::IndexSpace>(
    edge: AssetEdge<S>,
    hint: &'a Option<String>,
) -> Option<&'a str> {
    edge.is_bound()
        .then(|| hint.as_deref())
        .flatten()
        .filter(|name| !name.is_empty())
}

impl WeaponCombatFx {
    pub fn view_flash_present(&self) -> Option<&str> {
        present_bound(self.view_flash, &self.view_flash_hint)
    }

    pub fn world_flash_present(&self) -> Option<&str> {
        present_bound(self.world_flash, &self.world_flash_hint)
    }

    pub fn flash_present(&self, player_view: bool) -> Option<&str> {
        if player_view {
            self.view_flash_present()
        } else {
            self.world_flash_present()
        }
    }

    pub fn flash_edge(&self, player_view: bool) -> AssetEdge<FxSpace> {
        if player_view {
            self.view_flash
        } else {
            self.world_flash
        }
    }

    pub fn view_shell_eject_present(&self) -> Option<&str> {
        present_bound(self.view_shell_eject, &self.view_shell_eject_hint)
    }

    pub fn world_shell_eject_present(&self) -> Option<&str> {
        present_bound(self.world_shell_eject, &self.world_shell_eject_hint)
    }

    pub fn brass_present(&self, player_view: bool) -> Option<&str> {
        if player_view {
            self.view_shell_eject_present()
        } else {
            self.world_shell_eject_present()
        }
    }

    pub fn last_shot_eject_pair_authored(&self) -> bool {
        self.last_shot_eject_pair_authored
    }

    pub fn last_shot_eject_present(&self, player_view: bool) -> Option<&str> {
        if player_view {
            present_bound(self.view_last_shot_eject, &self.view_last_shot_eject_hint)
        } else {
            present_bound(self.world_last_shot_eject, &self.world_last_shot_eject_hint)
        }
    }

    pub fn brass_present_for_event(&self, player_view: bool, last_shot: bool) -> Option<&str> {
        if last_shot && self.last_shot_eject_pair_authored() {
            self.last_shot_eject_present(player_view)
        } else {
            self.brass_present(player_view)
        }
    }

    pub fn brass_edge(&self, player_view: bool) -> AssetEdge<FxSpace> {
        if player_view {
            self.view_shell_eject
        } else {
            self.world_shell_eject
        }
    }

    pub fn brass_edge_for_event(&self, player_view: bool, last_shot: bool) -> AssetEdge<FxSpace> {
        if last_shot && self.last_shot_eject_pair_authored() {
            if player_view {
                self.view_last_shot_eject
            } else {
                self.world_last_shot_eject
            }
        } else {
            self.brass_edge(player_view)
        }
    }

    pub fn explosion_present(&self) -> Option<&str> {
        present_bound(self.explosion, &self.explosion_hint)
    }

    pub fn fx_edges(&self) -> [AssetEdge<FxSpace>; 7] {
        [
            self.view_flash,
            self.world_flash,
            self.view_shell_eject,
            self.world_shell_eject,
            self.view_last_shot_eject,
            self.world_last_shot_eject,
            self.explosion,
        ]
    }
}

impl CatalogWeapon {
    pub fn with_timers(
        name: impl Into<String>,
        weap_def: Option<(u8, u32)>,
        gun_xmodel: Option<String>,
        sz_xanims: [Option<String>; WEAPON_ANIM_SLOTS],
        fire_time_ms: i32,
        raise_time_ms: i32,
        move_speed_scale: f32,
        ads_move_speed_scale: f32,
    ) -> Self {
        Self {
            name: name.into(),
            weap_def,
            display_name_key: None,
            reticle: WeaponReticleAssets::default(),
            hud_material_edges: WeaponHudMaterialEdges::default(),
            overlay_material: None,
            overlay_image: None,
            reticle_center_slot: None,
            reticle_side_slot: None,
            overlay_material_slot: None,
            scope_name: None,
            scope_rows: Default::default(),
            iw5_attachment_slots: std::array::from_fn(|_| None),
            iw5_reload_overrides: Vec::new(),
            iw5_anim_overrides: Vec::new(),
            iw5_fx_overrides: Vec::new(),
            iw5_notetrack_overrides: Vec::new(),
            hud_icon: None,
            hud_icon_slot: None,
            pickup_icon: None,
            pickup_icon_slot: None,
            pickup_icon_image: None,
            pickup_icon_ratio: 0,
            hud_icon_ratio: 0,
            hud_icon_image: None,
            dpad_icon: None,
            dpad_icon_image: None,
            dpad_icon_atlas: None,
            dpad_icon_ratio: 0,
            kill_icon: None,
            kill_icon_slot: None,
            kill_icon_image: None,
            proj_trail: None,
            proj_trail_slot: None,
            proj_beacon: None,
            proj_beacon_slot: None,
            proj_ignition: None,
            proj_ignition_slot: None,
            projectile_fx: WeaponProjectileFx::default(),
            gun_xmodel,
            hand_xmodel: None,
            world_model: None,
            projectile_model: None,
            rocket_model: None,
            sz_xanims,
            sz_xanims_right: [const { None }; WEAPON_ANIM_SLOTS],
            sz_xanims_left: [const { None }; WEAPON_ANIM_SLOTS],
            hide_tags: Vec::new(),
            sounds: WeaponSoundAliases::default(),
            combat_fx: WeaponCombatFx::default(),
            combat_slots: CombatFxSlots::default(),
            facts: WeaponBodyFacts {
                body_resolved: weap_def.is_some(),
                fire_time_ms,
                raise_time_ms,
                move_speed_scale,
                ads_move_speed_scale,
                ..WeaponBodyFacts::default()
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WeaponCatalog {
    entries: Vec<CatalogWeapon>,
    strings: ScriptStrings,
    iw5_attachments: HashMap<String, Iw5ScopeRow>,
}

impl WeaponCatalog {
    pub fn set_strings(&mut self, strings: ScriptStrings) {
        self.strings = strings;
    }

    pub fn capture(&mut self, stream: &ZoneStream<'_>) {
        let Some(geometry) = stream.weapon() else {
            return;
        };
        let Some(name_ptr) = geometry.name else {
            return;
        };
        let Ok(name) = stream.cstr(name_ptr) else {
            return;
        };
        if name.is_empty() {
            return;
        }
        let gun_xmodel = geometry
            .gun_xmodel_name
            .and_then(|ptr| stream.cstr(ptr).ok())
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let hand_xmodel = geometry
            .hand_xmodel_name
            .and_then(|ptr| stream.cstr(ptr).ok())
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let world_model = geometry
            .world_model_name
            .and_then(|ptr| stream.cstr(ptr).ok())
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let projectile_model = geometry
            .projectile_model_name
            .and_then(|ptr| stream.cstr(ptr).ok())
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let rocket_model = geometry
            .rocket_model_name
            .and_then(|ptr| stream.cstr(ptr).ok())
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let sz_xanims = geometry
            .sz_xanims
            .map(|arr| read_sz_xanims(stream, arr))
            .unwrap_or([const { None }; WEAPON_ANIM_SLOTS]);
        let sz_xanims_right = geometry
            .sz_xanims_right
            .map(|arr| read_sz_xanims(stream, arr))
            .unwrap_or([const { None }; WEAPON_ANIM_SLOTS]);
        let sz_xanims_left = geometry
            .sz_xanims_left
            .map(|arr| read_sz_xanims(stream, arr))
            .unwrap_or([const { None }; WEAPON_ANIM_SLOTS]);
        let hide_tags = read_hide_tags(stream, &self.strings, geometry.hide_tags);
        self.entries.push(CatalogWeapon {
            name: name.to_owned(),
            weap_def: geometry.weap_def.map(ptr_key),
            display_name_key: geometry
                .display_name_at_0x8
                .and_then(|ptr| read_name(stream, ptr)),
            reticle: WeaponReticleAssets {
                center_material: None,
                side_material: None,
                center_edge: AssetEdge::Absent,
                side_edge: AssetEdge::Absent,
                center_image: None,
                side_image: None,
                center_size: geometry.reticle_center_size_at_0x128,
                side_size: geometry.i_reticle_side_size,
                center_authored: geometry.reticle_center_material_slot.is_some(),
                side_authored: geometry.reticle_side_material_slot.is_some(),
            },
            hud_material_edges: WeaponHudMaterialEdges::default(),
            overlay_material: None,
            overlay_image: None,
            reticle_center_slot: geometry.reticle_center_material_slot,
            reticle_side_slot: geometry.reticle_side_material_slot,
            overlay_material_slot: geometry.overlay_material_slot,
            scope_name: None,
            scope_rows: Default::default(),
            iw5_attachment_slots: std::array::from_fn(|_| None),
            iw5_reload_overrides: Vec::new(),
            iw5_anim_overrides: Vec::new(),
            iw5_fx_overrides: Vec::new(),
            iw5_notetrack_overrides: Vec::new(),
            hud_icon: None,
            hud_icon_slot: geometry.hud_icon_slot,
            pickup_icon: None,
            pickup_icon_slot: geometry.pickup_icon_slot,
            pickup_icon_image: None,
            pickup_icon_ratio: geometry.pickup_icon_ratio,
            hud_icon_ratio: geometry.hud_icon_ratio,
            hud_icon_image: None,
            dpad_icon: geometry.dpad_icon_name.and_then(|p| read_name(stream, p)),
            dpad_icon_image: None,
            dpad_icon_atlas: None,
            dpad_icon_ratio: geometry.dpad_icon_ratio,
            kill_icon: geometry
                .kill_icon_name
                .and_then(|ptr| read_name(stream, ptr)),
            kill_icon_slot: geometry.kill_icon_slot,
            kill_icon_image: None,
            proj_trail: None,
            proj_trail_slot: geometry.proj_trail_slot,
            proj_beacon: None,
            proj_beacon_slot: geometry.proj_beacon_slot,
            proj_ignition: None,
            proj_ignition_slot: geometry.proj_ignition_slot,
            projectile_fx: WeaponProjectileFx::default(),
            gun_xmodel,
            hand_xmodel,
            world_model,
            projectile_model,
            rocket_model,
            sz_xanims,
            sz_xanims_right,
            sz_xanims_left,
            hide_tags,
            sounds: WeaponSoundAliases {
                notetrack_convention: NotetrackConvention::SoundMap,
                fire: geometry
                    .fire_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                fire_player: geometry
                    .fire_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                empty_fire: geometry
                    .empty_fire_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                empty_fire_player: geometry
                    .empty_fire_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                melee_swipe: geometry
                    .melee_swipe_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                melee_swipe_player: geometry
                    .melee_swipe_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                melee_hit: geometry
                    .melee_hit_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                melee_miss: geometry
                    .melee_miss_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                pickup: geometry
                    .pickup_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                pickup_player: geometry
                    .pickup_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                ammo_pickup: geometry
                    .ammo_pickup_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                ammo_pickup_player: geometry
                    .ammo_pickup_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                pullback: geometry
                    .pullback_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                pullback_player: geometry
                    .pullback_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload: geometry
                    .reload_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload_player: geometry
                    .reload_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload_empty: geometry
                    .reload_empty_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload_empty_player: geometry
                    .reload_empty_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload_start: geometry
                    .reload_start_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload_start_player: geometry
                    .reload_start_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload_end: geometry
                    .reload_end_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                reload_end_player: geometry
                    .reload_end_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                rechamber: geometry
                    .rechamber_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                rechamber_player: geometry
                    .rechamber_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                alt_switch: geometry
                    .alt_switch_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                alt_switch_player: geometry
                    .alt_switch_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                raise: geometry
                    .raise_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                raise_player: geometry
                    .raise_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                first_raise: geometry
                    .first_raise_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                first_raise_player: geometry
                    .first_raise_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                putaway: geometry
                    .putaway_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                putaway_player: geometry
                    .putaway_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                proj_explosion: geometry
                    .proj_explosion_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                projectile: geometry
                    .projectile_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                proj_ignition_sound: geometry
                    .proj_ignition_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                bounce: geometry
                    .bounce_sound_names
                    .map(|slot| slot.and_then(|ptr| read_name(stream, ptr))),
                notetrack_sound_map: read_script_string_map(
                    stream,
                    &self.strings,
                    geometry.notetrack_sound_keys,
                    geometry.notetrack_sound_values,
                ),
                notetrack_rumble_map: read_script_string_map(
                    stream,
                    &self.strings,
                    geometry.notetrack_rumble_keys,
                    geometry.notetrack_rumble_values,
                ),
                fire_player_akimbo: None,
                fire_loop: None,
                fire_loop_player: None,
                fire_stop: None,
                fire_stop_player: None,
                fire_last: geometry
                    .fire_last_sound_name
                    .and_then(|ptr| read_name(stream, ptr)),
                fire_last_player: geometry
                    .fire_last_sound_player_name
                    .and_then(|ptr| read_name(stream, ptr)),
                leftover_sound_overrides: Vec::new(),
                fire_ptr_kind: None,
                fire_player_ptr_kind: None,
                reload_player_ptr_kind: None,
            },
            combat_slots: CombatFxSlots {
                view_flash: geometry.view_flash_slot,
                world_flash: geometry.world_flash_slot,
                view_shell_eject: geometry.view_shell_eject_slot,
                world_shell_eject: geometry.world_shell_eject_slot,
                view_last_shot_eject: geometry.view_last_shot_eject_slot,
                world_last_shot_eject: geometry.world_last_shot_eject_slot,
                explosion: geometry.explosion_slot,
                tracer: geometry.tracer_slot,
            },
            combat_fx: WeaponCombatFx {
                last_shot_eject_pair_authored: geometry.view_last_shot_eject_slot.is_some()
                    && geometry.world_last_shot_eject_slot.is_some(),
                ..WeaponCombatFx::default()
            },
            facts: WeaponBodyFacts {
                body_resolved: geometry.weap_def.is_some(),
                fire_time_ms: geometry.fire_time_ms,
                impact_type: geometry.impact_type,
                raise_time_ms: geometry.raise_time_ms,
                drop_time_ms: geometry.drop_time_ms,
                fire_delay_ms: geometry.fire_delay_ms,
                hold_fire_time_ms: geometry.hold_fire_time_ms,
                weap_type: geometry.weap_type,
                weap_class: geometry.weap_class,
                player_anim_type: geometry.player_anim_type,
                offhand_class: geometry.offhand_class,
                shots_per_fire: geometry.shots_per_fire,
                ammo_index: geometry.ammo_index,
                clip_index: geometry.clip_index,
                ammo_counter_clip: geometry.ammo_counter_clip,
                low_ammo_warning_threshold: geometry.low_ammo_warning_threshold,
                hip_spread_stand_min: geometry.hip_spread_stand_min,
                hip_spread_ducked_min: geometry.hip_spread_ducked_min,
                hip_spread_prone_min: geometry.hip_spread_prone_min,
                hip_spread_stand_max: geometry.hip_spread_stand_max,
                hip_spread_ducked_max: geometry.hip_spread_ducked_max,
                hip_spread_prone_max: geometry.hip_spread_prone_max,
                hip_spread_decay_rate: geometry.hip_spread_decay_rate,
                hip_spread_fire_add: geometry.hip_spread_fire_add,
                hip_spread_turn_add: geometry.hip_spread_turn_add,
                hip_spread_move_add: geometry.hip_spread_move_add,
                hip_spread_ducked_decay: geometry.hip_spread_ducked_decay,
                hip_spread_prone_decay: geometry.hip_spread_prone_decay,
                i_reticle_side_size: geometry.i_reticle_side_size,
                i_reticle_min_ofs: geometry.i_reticle_min_ofs,
                hip_reticle_side_pos: geometry.hip_reticle_side_pos,
                ads_aim_pitch: geometry.ads_aim_pitch,
                ads_crosshair_in_frac: geometry.ads_crosshair_in_frac,
                ads_crosshair_out_frac: geometry.ads_crosshair_out_frac,
                ads_spread: geometry.ads_spread,
                aim_down_sight: geometry.aim_down_sight,
                ads_zoom_fov: geometry.ads_zoom_fov,
                ads_dof: Some(geometry.ads_dof),
                ads_zoom_in_frac: geometry.ads_zoom_in_frac,
                ads_zoom_out_frac: geometry.ads_zoom_out_frac,
                no_ads_when_mag_empty: geometry.no_ads_when_mag_empty,
                inherits_perks: geometry.inherits_perks,
                ads_in_rate: geometry.ads_in_rate,
                ads_out_rate: geometry.ads_out_rate,
                rechamber_while_ads: geometry.rechamber_while_ads,
                ads_fire_only: geometry.ads_fire_only,
                melee_damage: geometry.melee_damage,
                overlay_reticle: geometry.overlay_reticle,
                overlay_interface: geometry.overlay_interface,
                ads_overlay_width: geometry.ads_overlay_width,
                ads_overlay_height: geometry.ads_overlay_height,
                melee_time_ms: geometry.melee_time_ms,
                melee_delay_ms: geometry.melee_delay_ms,
                melee_charge_time_ms: geometry.melee_charge_time_ms,
                melee_charge_delay_ms: geometry.melee_charge_delay_ms,
                knife_model: geometry.knife_model,
                quick_raise_time_ms: geometry.quick_raise_time_ms,
                quick_drop_time_ms: geometry.quick_drop_time_ms,
                select_requires_ammo_at_0x667: geometry.select_requires_ammo_at_0x667,
                offhand_hold_is_cancelable_at_0x681: geometry.offhand_hold_is_cancelable_at_0x681,
                move_speed_scale: geometry.move_speed_scale,
                ads_move_speed_scale: geometry.ads_move_speed_scale,
                sprint_duration_scale: geometry.sprint_duration_scale,
                stance_ofs_at_0x168: geometry.stance_ofs_at_0x168,
                stance_ofs_at_0x18c: geometry.stance_ofs_at_0x18c,
                night_vision_wear_time: geometry.night_vision_wear_time,
                ads_bob_factor_at_0x330: geometry.ads_bob_factor_at_0x330,
                ads_view_bob_mult_at_0x334: geometry.ads_view_bob_mult_at_0x334,
                movement: movement_from_capture(geometry.movement),
                idle: idle_from_capture(geometry.idle),
                clip_size: geometry.clip_size,
                penetrate_type: geometry.penetrate_type,
                penetrate_multiplier: geometry.penetrate_multiplier,
                motion_tracker: geometry.motion_tracker,
                rifle_bullet: geometry.rifle_bullet,
                inventory_type: geometry.inventory_type,
                fire_type: geometry.fire_type,
                max_ammo: geometry.max_ammo,
                damage: geometry.damage,
                rechamber_time_ms: geometry.rechamber_time_ms,
                rechamber_bolt_time_ms: geometry.rechamber_bolt_time_ms,
                rechamber_bolt_delay_ms: geometry.rechamber_bolt_delay_ms,
                reload_time_ms: geometry.reload_time_ms,
                reload_show_rocket_time_ms: geometry.reload_show_rocket_time_ms,
                reload_empty_time_ms: geometry.reload_empty_time_ms,
                reload_add_time_ms: geometry.reload_add_time_ms,
                reload_empty_add_time_ms: 0,
                reload_start_time_ms: geometry.reload_start_time_ms,
                reload_start_add_time_ms: geometry.reload_start_add_time_ms,
                reload_end_time_ms: geometry.reload_end_time_ms,
                dual_mag: None,
                kill_icon_ratio: geometry.kill_icon_ratio,
                flip_kill_icon: geometry.flip_kill_icon,
                reload_ammo_add: geometry.reload_ammo_add,
                reload_start_add: geometry.reload_start_add,
                no_partial_reload: geometry.no_partial_reload,
                bolt_action: geometry.bolt_action,
                segmented_reload: geometry.segmented_reload,
                sprint_raise_time_ms: geometry.sprint_raise_time_ms,
                sprint_loop_time_ms: geometry.sprint_loop_time_ms,
                sprint_drop_time_ms: geometry.sprint_drop_time_ms,
                fuse_time_ms: geometry.fuse_time_ms,
                cook_off_hold: geometry.cook_off_hold,
                clip_only: geometry.clip_only,
                timed_detonation: geometry.timed_detonation,
                proj_impact_explode: geometry.proj_impact_explode,
                stick_to_players: geometry.stick_to_players,
                explosion_radius: geometry.explosion_radius,
                explosion_radius_min: geometry.explosion_radius_min,
                explosion_inner_damage: geometry.explosion_inner_damage,
                explosion_outer_damage: geometry.explosion_outer_damage,
                projectile_speed: geometry.projectile_speed,
                projectile_speed_up: geometry.projectile_speed_up,
                projectile_speed_forward: geometry.projectile_speed_forward,
                projectile_activate_dist: geometry.projectile_activate_dist,
                projectile_explosion_type: geometry.projectile_explosion_type,
                parallel_bounce: geometry.parallel_bounce,
                perpendicular_bounce: geometry.perpendicular_bounce,
                location_damage_mult: geometry.location_damage_mult,
                start_ammo: geometry.start_ammo,
                ammo_count_clip_relative: false,
                min_damage: geometry.min_damage,
                min_player_damage: geometry.min_player_damage,
                max_damage_range: geometry.max_damage_range,
                min_damage_range: geometry.min_damage_range,
                kick: WeaponKickFacts::from_capture(geometry.kick),
                sway: WeaponSwayFacts::from_capture(geometry.sway),
                dual_wield_view_model_offset: geometry.dual_wield_view_model_offset,
                no_dual_wield: geometry.no_dual_wield,
            },
        });
    }

    pub fn resolve_reticles(&mut self, materials: &crate::MaterialCatalog) {
        let names_of = |slot: Ptr| {
            let material = materials
                .material_index(slot)
                .and_then(|i| materials.materials.get(i.get()))?;
            let name = Some(material.name.to_string()).filter(|n| !n.is_empty())?;
            let image = materials.hud_image_name(material).map(str::to_owned);
            Some((name, image))
        };
        for entry in &mut self.entries {
            if let Some(slot) = entry.reticle_center_slot {
                if let Some((name, image)) = names_of(slot) {
                    entry.reticle.center_material = Some(name);
                    entry.reticle.center_image = image;
                }
            }
            if entry.reticle.center_image.is_none() {
                if let Some(name) = entry.reticle.center_material.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.reticle.center_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if let Some(slot) = entry.reticle_side_slot {
                if let Some((name, image)) = names_of(slot) {
                    entry.reticle.side_material = Some(name);
                    entry.reticle.side_image = image;
                }
            }
            if entry.reticle.side_image.is_none() {
                if let Some(name) = entry.reticle.side_material.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.reticle.side_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if let Some(slot) = entry.overlay_material_slot {
                if let Some((name, image)) = names_of(slot) {
                    entry.overlay_material = Some(name);
                    entry.overlay_image = image;
                }
            }
            if entry.overlay_image.is_none() {
                if let Some(name) = entry.overlay_material.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.overlay_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if let Some(slot) = entry.pickup_icon_slot {
                if let Some((name, image)) = names_of(slot) {
                    entry.pickup_icon = Some(name);
                    entry.pickup_icon_image = image;
                }
            }
            if let Some(slot) = entry.hud_icon_slot {
                if let Some((name, image)) = names_of(slot) {
                    entry.hud_icon = Some(name);
                    if entry.hud_icon_image.is_none() {
                        entry.hud_icon_image = image;
                    }
                }
            }
            if entry.hud_icon_image.is_none() {
                if let Some(name) = entry.hud_icon.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.hud_icon_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if let Some(slot) = entry.kill_icon_slot {
                if let Some((name, image)) = names_of(slot) {
                    if entry.kill_icon.is_none() {
                        entry.kill_icon = Some(name);
                    }
                    if entry.kill_icon_image.is_none() {
                        entry.kill_icon_image = image;
                    }
                }
            }
            if entry.kill_icon_image.is_none() {
                if let Some(name) = entry.kill_icon.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.kill_icon_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if let Some(name) = entry.dpad_icon.as_deref()
                && let Some(index) = materials.material_index_by_name(name)
                && let Some(material) = materials.materials.get(index.order())
            {
                entry.dpad_icon_image = materials.hud_image_name(material).map(str::to_owned);
                entry.dpad_icon_atlas = material.texture_atlas;
            }
            entry.reticle.center_edge = material_hint_edge(
                entry.reticle.center_material.as_deref(),
                entry.reticle.center_authored,
                materials,
            );
            entry.reticle.side_edge = material_hint_edge(
                entry.reticle.side_material.as_deref(),
                entry.reticle.side_authored,
                materials,
            );
            entry.hud_material_edges.overlay = material_hint_edge(
                entry.overlay_material.as_deref(),
                entry.overlay_material_slot.is_some(),
                materials,
            );
            entry.hud_material_edges.hud_icon = material_hint_edge(
                entry.hud_icon.as_deref(),
                entry.hud_icon_slot.is_some(),
                materials,
            );
            entry.hud_material_edges.pickup_icon = material_hint_edge(
                entry.pickup_icon.as_deref(),
                entry.pickup_icon_slot.is_some(),
                materials,
            );
            entry.hud_material_edges.kill_icon = material_hint_edge(
                entry.kill_icon.as_deref(),
                entry.kill_icon_slot.is_some(),
                materials,
            );
        }
    }

    /// The half of reticle resolution that still has an answer once the walk is
    /// over: the slot lookups need the zone's pointer map, the name lookups do
    /// not. Running the slot half against a finished population was reading a
    /// map that `finalize` had already emptied, so every one of them was `None`.
    pub fn resolve_reticle_images(&mut self, materials: &crate::MaterialDefinitions) {
        for entry in &mut self.entries {
            if entry.reticle.center_image.is_none() {
                if let Some(name) = entry.reticle.center_material.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.reticle.center_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if entry.reticle.side_image.is_none() {
                if let Some(name) = entry.reticle.side_material.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.reticle.side_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if entry.overlay_image.is_none() {
                if let Some(name) = entry.overlay_material.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.overlay_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if entry.hud_icon_image.is_none() {
                if let Some(name) = entry.hud_icon.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.hud_icon_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if entry.kill_icon_image.is_none() {
                if let Some(name) = entry.kill_icon.as_deref() {
                    if let Some(index) = materials.material_index_by_name(name) {
                        if let Some(material) = materials.materials.get(index.order()) {
                            entry.kill_icon_image =
                                materials.hud_image_name(material).map(str::to_owned);
                        }
                    }
                }
            }
            if let Some(name) = entry.dpad_icon.as_deref()
                && let Some(index) = materials.material_index_by_name(name)
                && let Some(material) = materials.materials.get(index.order())
            {
                entry.dpad_icon_image = materials.hud_image_name(material).map(str::to_owned);
                entry.dpad_icon_atlas = material.texture_atlas;
            }
            entry.reticle.center_edge = material_hint_edge(
                entry.reticle.center_material.as_deref(),
                entry.reticle.center_authored,
                materials,
            );
            entry.reticle.side_edge = material_hint_edge(
                entry.reticle.side_material.as_deref(),
                entry.reticle.side_authored,
                materials,
            );
            entry.hud_material_edges.overlay = material_hint_edge(
                entry.overlay_material.as_deref(),
                entry.overlay_material_slot.is_some(),
                materials,
            );
            entry.hud_material_edges.hud_icon = material_hint_edge(
                entry.hud_icon.as_deref(),
                entry.hud_icon_slot.is_some(),
                materials,
            );
            entry.hud_material_edges.pickup_icon = material_hint_edge(
                entry.pickup_icon.as_deref(),
                entry.pickup_icon_slot.is_some(),
                materials,
            );
            entry.hud_material_edges.kill_icon = material_hint_edge(
                entry.kill_icon.as_deref(),
                entry.kill_icon_slot.is_some(),
                materials,
            );
        }
    }

    pub fn hud_material_edge_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for entry in &self.entries {
            census.push(entry.reticle.center_edge);
            census.push(entry.reticle.side_edge);
            census.push(entry.hud_material_edges.overlay);
            census.push(entry.hud_material_edges.hud_icon);
            census.push(entry.hud_material_edges.pickup_icon);
            census.push(entry.hud_material_edges.kill_icon);
        }
        census
    }

    pub fn resolve_projectile_fx_edges(&mut self, fx: &crate::FxCatalog) {
        for entry in &mut self.entries {
            stamp_fx_edge(
                entry.proj_trail_slot,
                fx,
                &mut entry.projectile_fx.trail,
                &mut entry.proj_trail,
            );
            stamp_fx_edge(
                entry.proj_beacon_slot,
                fx,
                &mut entry.projectile_fx.beacon,
                &mut entry.proj_beacon,
            );
            stamp_fx_edge(
                entry.proj_ignition_slot,
                fx,
                &mut entry.projectile_fx.ignition,
                &mut entry.proj_ignition,
            );
        }
    }

    pub fn projectile_fx_edge_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for entry in &self.entries {
            for edge in entry.projectile_fx.edges() {
                census.push(edge);
            }
        }
        census
    }

    pub fn resolve_combat_fx(&mut self, fx: &crate::FxCatalog, tracers: &crate::TracerCatalog) {
        for entry in &mut self.entries {
            stamp_combat_fx(&mut entry.combat_fx, entry.combat_slots, fx, tracers);
        }
    }

    pub fn push(&mut self, entry: CatalogWeapon) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn tracer_type_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for entry in &self.entries {
            census.push(entry.combat_fx.tracer);
        }
        census
    }

    pub fn combat_fx_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for entry in &self.entries {
            for edge in entry.combat_fx.fx_edges() {
                census.push(edge);
            }
        }
        census
    }

    pub fn projectile_model_hints(&self) -> HashSet<String> {
        self.entries
            .iter()
            .filter_map(|entry| entry.projectile_model.clone())
            .filter(|name| !name.is_empty())
            .collect()
    }

    pub fn rocket_model_hints(&self) -> HashSet<String> {
        self.entries
            .iter()
            .filter_map(|entry| entry.rocket_model.clone())
            .collect()
    }

    pub fn into_build(self) -> WeaponBuild {
        WeaponBuild::from_catalog(self.entries, self.iw5_attachments)
    }
}

fn material_hint_edge(
    hint: Option<&str>,
    authored_slot: bool,
    materials: &crate::MaterialDefinitions,
) -> AssetEdge<MaterialSpace> {
    let hint = hint.filter(|name| !name.is_empty());
    if !authored_slot && hint.is_none() {
        return AssetEdge::Absent;
    }
    match hint.and_then(|name| materials.material_index_by_name(name)) {
        Some(index) => AssetEdge::bind(index, materials.zone_of(index.order())),
        None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
    }
}

fn stamp_fx_edge(
    slot: Option<Ptr>,
    fx: &crate::FxCatalog,
    edge: &mut AssetEdge<FxSpace>,
    hint: &mut Option<String>,
) {
    let leftover = hint.as_deref().filter(|s| !s.is_empty()).map(str::to_owned);
    let slot_name = slot.and_then(|s| fx.name_at_slot(s)).map(str::to_owned);
    let name = leftover.or(slot_name);
    *hint = name.clone();
    *edge = match name.as_deref() {
        None if slot.is_none() => AssetEdge::Absent,
        Some(name) => match fx.index_by_name(name) {
            Some(index) => AssetEdge::bind_order(index, fx.zone_of(index)),
            None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
        },
        None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
    };
}

fn stamp_combat_fx(
    combat: &mut WeaponCombatFx,
    slots: CombatFxSlots,
    fx: &crate::FxCatalog,
    tracers: &crate::TracerCatalog,
) {
    stamp_fx_edge(
        slots.view_flash,
        fx,
        &mut combat.view_flash,
        &mut combat.view_flash_hint,
    );
    stamp_fx_edge(
        slots.world_flash,
        fx,
        &mut combat.world_flash,
        &mut combat.world_flash_hint,
    );
    stamp_fx_edge(
        slots.view_shell_eject,
        fx,
        &mut combat.view_shell_eject,
        &mut combat.view_shell_eject_hint,
    );
    stamp_fx_edge(
        slots.world_shell_eject,
        fx,
        &mut combat.world_shell_eject,
        &mut combat.world_shell_eject_hint,
    );
    stamp_fx_edge(
        slots.view_last_shot_eject,
        fx,
        &mut combat.view_last_shot_eject,
        &mut combat.view_last_shot_eject_hint,
    );
    stamp_fx_edge(
        slots.world_last_shot_eject,
        fx,
        &mut combat.world_last_shot_eject,
        &mut combat.world_last_shot_eject_hint,
    );
    stamp_fx_edge(
        slots.explosion,
        fx,
        &mut combat.explosion,
        &mut combat.explosion_hint,
    );
    let tracer_name = slots.tracer.and_then(|s| tracers.name_at_slot(s));
    combat.tracer_hint = tracer_name.map(str::to_owned);
    combat.tracer = match (slots.tracer, tracer_name) {
        (None, _) => AssetEdge::Absent,
        (_, Some(name)) => match tracers.index_by_name(name) {
            Some(index) => AssetEdge::bind_order(index, tracers.zone_of(index)),
            None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
        },
        (Some(_), None) => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
    };
    combat.last_shot_eject_pair_authored = slots.last_shot_pair_authored();
}

fn ptr_key(p: Ptr) -> (u8, u32) {
    (p.block, p.offset)
}

fn leftover_hip_spread_block(read: impl Fn(usize) -> f32, stand_min: usize) -> [f32; 12] {
    core::array::from_fn(|i| read(stand_min + i * 4))
}

fn apply_leftover_hip_spread(facts: &mut WeaponBodyFacts, block: [f32; 12]) {
    facts.hip_spread_stand_min = block[0];
    facts.hip_spread_ducked_min = block[1];
    facts.hip_spread_prone_min = block[2];
    facts.hip_spread_stand_max = block[3];
    facts.hip_spread_ducked_max = block[4];
    facts.hip_spread_prone_max = block[5];
    facts.hip_spread_decay_rate = block[6];
    facts.hip_spread_fire_add = block[7];
    facts.hip_spread_turn_add = block[8];
    facts.hip_spread_move_add = block[9];
    facts.hip_spread_ducked_decay = block[10];
    facts.hip_spread_prone_decay = block[11];
}

fn read_name(stream: &ZoneStream<'_>, ptr: Ptr) -> Option<String> {
    stream
        .cstr(ptr)
        .ok()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

fn iw5_anim_tree_type_to_iw4_slot(ty: u32) -> Option<usize> {
    let ty = ty as usize;
    if ty <= weap_anim::ADS_RECHAMBER {
        Some(ty)
    } else if ty == fastfile_iw5::size::WEAPON_ANIM_ADS_UP {
        Some(weap_anim::ADS_UP)
    } else if ty == fastfile_iw5::size::WEAPON_ANIM_ADS_DOWN {
        Some(weap_anim::ADS_DOWN)
    } else {
        None
    }
}

pub fn overlay_name_is_hud_iris(name: &str) -> bool {
    !name.starts_with("mc/")
}

fn read_hide_tags(
    stream: &ZoneStream<'_>,
    strings: &ScriptStrings,
    arr: Option<Ptr>,
) -> Vec<String> {
    let Some(arr) = arr else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..32usize {
        let Ok(id) = stream.u16_at(arr, i * 2) else {
            break;
        };
        if id == 0 {
            continue;
        }
        if let Some(name) = strings.get(stream, id) {
            if !name.is_empty() {
                out.push(name.to_owned());
            }
        }
    }
    out
}

fn read_script_string_map(
    stream: &ZoneStream<'_>,
    strings: &ScriptStrings,
    keys: Option<Ptr>,
    values: Option<Ptr>,
) -> Vec<(String, String)> {
    let (Some(keys), Some(values)) = (keys, values) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..16usize {
        let Ok(key_id) = stream.u16_at(keys, i * 2) else {
            break;
        };
        if key_id == 0 {
            break;
        }
        let Ok(val_id) = stream.u16_at(values, i * 2) else {
            break;
        };
        let Some(key) = strings.get(stream, key_id).filter(|s| !s.is_empty()) else {
            continue;
        };
        let val = strings
            .get(stream, val_id)
            .filter(|s| !s.is_empty())
            .unwrap_or(key);
        out.push((key.to_owned(), val.to_owned()));
    }
    out
}

fn read_sz_xanims(stream: &ZoneStream<'_>, arr: Ptr) -> [Option<String>; WEAPON_ANIM_SLOTS] {
    let mut out = [const { None }; WEAPON_ANIM_SLOTS];
    for (i, slot) in out.iter_mut().take(WEAPON_ANIM_COUNT).enumerate() {
        let name_ptr = match stream.ptr_at(arr, i * stream.pointer_bytes()) {
            Ok(ZonePtr::Offset(q)) => Some(stream.resolve_alias(q)),
            _ => None,
        };
        *slot = name_ptr
            .and_then(|ptr| stream.cstr(ptr).ok())
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
    }
    out
}

fn movement_from_capture(c: WeaponMovementOfsCapture) -> WeaponMovementOfsInputs {
    WeaponMovementOfsInputs {
        stand_move_at_0x138: c.stand_move_at_0x138,
        stand_rot_at_0x144: c.stand_rot_at_0x144,
        strafe_move_at_0x150: c.strafe_move_at_0x150,
        strafe_rot_at_0x15c: c.strafe_rot_at_0x15c,
        ducked_move_at_0x174: c.ducked_move_at_0x174,
        ducked_rot_at_0x180: c.ducked_rot_at_0x180,
        prone_move_at_0x198: c.prone_move_at_0x198,
        prone_rot_at_0x1a4: c.prone_rot_at_0x1a4,
        pos_move_rate_at_0x1b0: c.pos_move_rate_at_0x1b0,
        pos_prone_move_rate_at_0x1b4: c.pos_prone_move_rate_at_0x1b4,
        stand_move_min_speed_at_0x1b8: c.stand_move_min_speed_at_0x1b8,
        ducked_move_min_speed_at_0x1bc: c.ducked_move_min_speed_at_0x1bc,
        prone_move_min_speed_at_0x1c0: c.prone_move_min_speed_at_0x1c0,
        pos_rot_rate_at_0x1c4: c.pos_rot_rate_at_0x1c4,
        pos_prone_rot_rate_at_0x1c8: c.pos_prone_rot_rate_at_0x1c8,
    }
}

fn idle_from_capture(c: WeaponIdleCapture) -> WeaponIdleInputs {
    WeaponIdleInputs {
        ads_idle_amount_at_0x36c: c.ads_idle_amount_at_0x36c,
        hip_idle_amount_at_0x370: c.hip_idle_amount_at_0x370,
        ads_idle_speed_at_0x374: c.ads_idle_speed_at_0x374,
        hip_idle_speed_at_0x378: c.hip_idle_speed_at_0x378,
        idle_crouch_factor_at_0x37c: c.idle_crouch_factor_at_0x37c,
        idle_prone_factor_at_0x380: c.idle_prone_factor_at_0x380,
    }
}
