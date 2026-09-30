use super::*;

impl WeaponCatalog {
    pub fn capture_t5(
        &mut self,
        stream: &fastfile_t5::ZoneStream<'_>,
        strings: &fastfile_t5::ScriptStrings,
    ) {
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
        let sz_xanims = geometry
            .sz_xanims
            .map(|arr| read_sz_xanims_t5(stream, arr))
            .unwrap_or([const { None }; WEAPON_ANIM_SLOTS]);
        self.entries.push(CatalogWeapon {
            reticle_center_slot: leftover_t5_asset_slot(
                stream,
                geometry.weap_def,
                fastfile_t5::size::WEAPON_DEF_RETICLE_CENTER_OFF,
            ),
            reticle_side_slot: leftover_t5_asset_slot(
                stream,
                geometry.weap_def,
                fastfile_t5::size::WEAPON_DEF_RETICLE_SIDE_OFF,
            ),
            name: name.to_owned(),
            weap_def: geometry.weap_def.map(|p| (p.block, p.offset)),
            display_name_key: geometry
                .display_name
                .and_then(|ptr| stream.cstr(ptr).ok())
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
            reticle: leftover_t5_reticle(stream, geometry.weap_def),
            hud_material_edges: WeaponHudMaterialEdges::default(),
            overlay_material: leftover_t5_overlay_name(stream, &geometry),
            overlay_image: None,
            overlay_material_slot: leftover_t5_overlay_slot(stream, &geometry),
            scope_name: None,
            scope_rows: Default::default(),
            iw5_attachment_slots: std::array::from_fn(|_| None),
            iw5_reload_overrides: Vec::new(),
            iw5_anim_overrides: Vec::new(),
            iw5_fx_overrides: Vec::new(),
            iw5_notetrack_overrides: Vec::new(),
            hud_icon: leftover_t5_material_name_opt(
                stream,
                geometry.weap_def,
                fastfile_t5::size::WEAPON_DEF_HUD_ICON_OFF,
            ),
            hud_icon_slot: leftover_t5_asset_slot(
                stream,
                geometry.weap_def,
                fastfile_t5::size::WEAPON_DEF_HUD_ICON_OFF,
            ),

            pickup_icon: None,
            pickup_icon_slot: None,
            pickup_icon_image: None,
            pickup_icon_ratio: 0,
            hud_icon_ratio: geometry
                .weap_def
                .map_or(0, |body| i32_at_t5(stream, body, 0x324)),
            hud_icon_image: None,
            dpad_icon: None,
            dpad_icon_image: None,
            dpad_icon_atlas: None,
            dpad_icon_ratio: 0,
            kill_icon: leftover_t5_material_name_opt(
                stream,
                geometry.weap_def,
                fastfile_t5::size::WEAPON_DEF_KILL_ICON_OFF,
            ),
            kill_icon_slot: leftover_t5_asset_slot(
                stream,
                geometry.weap_def,
                fastfile_t5::size::WEAPON_DEF_KILL_ICON_OFF,
            ),
            kill_icon_image: None,
            proj_trail: None,
            proj_trail_slot: None,
            proj_beacon: None,
            proj_beacon_slot: None,
            proj_ignition: None,
            proj_ignition_slot: None,
            projectile_fx: WeaponProjectileFx::default(),
            gun_xmodel,
            hand_xmodel,

            world_model: geometry
                .world_model_name
                .and_then(|ptr| stream.cstr(ptr).ok())
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            projectile_model: geometry
                .projectile_model_name
                .and_then(|ptr| stream.cstr(ptr).ok())
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            rocket_model: geometry
                .rocket_model_name
                .and_then(|ptr| stream.cstr(ptr).ok())
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            sz_xanims,
            sz_xanims_right: [const { None }; WEAPON_ANIM_SLOTS],
            sz_xanims_left: [const { None }; WEAPON_ANIM_SLOTS],
            hide_tags: read_hide_tags_t5(stream, strings, geometry.hide_tags),
            sounds: leftover_t5_sounds(stream, strings, &geometry),
            combat_fx: WeaponCombatFx::default(),
            combat_slots: CombatFxSlots::default(),
            facts: capture_t5_body_facts(stream, &geometry),
        });
        let last = self.entries.last_mut().expect("just pushed");
        let (fx, slots) = leftover_t5_combat_fx(stream, &geometry);
        last.combat_fx = fx;
        last.combat_slots = slots;
    }
}

fn remap_t5_weap_class(raw: i32) -> i32 {
    match raw {
        0 => 0,
        1 => 2,
        2 => 3,
        3 => 4,
        4 => 5,
        5 => 6,
        6 => 7,
        7 => 8,
        8 => 10,
        10 => 11,
        other => other,
    }
}

fn read_sz_xanims_t5(
    stream: &fastfile_t5::ZoneStream<'_>,
    arr: fastfile_t5::Ptr,
) -> [Option<String>; WEAPON_ANIM_SLOTS] {
    let mut t5 = [const { None }; fastfile_t5::size::WEAPON_XANIM_COUNT];
    for (i, slot) in t5.iter_mut().enumerate() {
        let name_ptr = match stream.ptr_at(arr, i * 4) {
            Ok(fastfile_t5::ZonePtr::Offset(q)) => Some(stream.resolve_alias(q)),
            _ => None,
        };
        *slot = name_ptr
            .and_then(|ptr| stream.cstr(ptr).ok())
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
    }
    remap_t5_sz_xanims(&t5)
}

fn remap_t5_sz_xanims(t5: &[Option<String>]) -> [Option<String>; WEAPON_ANIM_SLOTS] {
    use fastfile_t5::size::weap_anim as t5_anim;
    const PAIRS: [(usize, usize); 34] = [
        (t5_anim::IDLE, weap_anim::IDLE),
        (t5_anim::EMPTY_IDLE, weap_anim::EMPTY_IDLE),
        (t5_anim::FIRE, weap_anim::FIRE),
        (t5_anim::HOLD_FIRE, weap_anim::HOLD_FIRE),
        (t5_anim::LASTSHOT, weap_anim::LASTSHOT),
        (t5_anim::RECHAMBER, weap_anim::RECHAMBER),
        (t5_anim::MELEE, weap_anim::MELEE),
        (t5_anim::MELEE_CHARGE, weap_anim::MELEE_CHARGE),
        (t5_anim::RELOAD, weap_anim::RELOAD),
        (t5_anim::RELOAD_EMPTY, weap_anim::RELOAD_EMPTY),
        (t5_anim::RELOAD_START, weap_anim::RELOAD_START),
        (t5_anim::RELOAD_END, weap_anim::RELOAD_END),
        (t5_anim::RELOAD_QUICK, weap_anim_extra::RELOAD_QUICK),
        (
            t5_anim::RELOAD_QUICK_EMPTY,
            weap_anim_extra::RELOAD_QUICK_EMPTY,
        ),
        (t5_anim::RAISE, weap_anim::RAISE),
        (t5_anim::FIRST_RAISE, weap_anim::FIRST_RAISE),
        (t5_anim::DROP, weap_anim::DROP),
        (t5_anim::ALT_RAISE, weap_anim::ALT_RAISE),
        (t5_anim::ALT_DROP, weap_anim::ALT_DROP),
        (t5_anim::QUICK_RAISE, weap_anim::QUICK_RAISE),
        (t5_anim::QUICK_DROP, weap_anim::QUICK_DROP),
        (t5_anim::EMPTY_RAISE, weap_anim::EMPTY_RAISE),
        (t5_anim::EMPTY_DROP, weap_anim::EMPTY_DROP),
        (t5_anim::SPRINT_IN, weap_anim::SPRINT_IN),
        (t5_anim::SPRINT_LOOP, weap_anim::SPRINT_LOOP),
        (t5_anim::SPRINT_OUT, weap_anim::SPRINT_OUT),
        (t5_anim::DETONATE, weap_anim::DETONATE),
        (t5_anim::NIGHTVISION_WEAR, weap_anim::NIGHTVISION_WEAR),
        (t5_anim::NIGHTVISION_REMOVE, weap_anim::NIGHTVISION_REMOVE),
        (t5_anim::ADS_FIRE, weap_anim::ADS_FIRE),
        (t5_anim::ADS_LASTSHOT, weap_anim::ADS_LASTSHOT),
        (t5_anim::ADS_RECHAMBER, weap_anim::ADS_RECHAMBER),
        (t5_anim::ADS_UP, weap_anim::ADS_UP),
        (t5_anim::ADS_DOWN, weap_anim::ADS_DOWN),
    ];
    let mut out = [const { None }; WEAPON_ANIM_SLOTS];
    for (src, dst) in PAIRS {
        if src < t5.len() {
            out[dst] = t5[src].clone();
        }
    }
    out
}

fn t5_to_iw4_ptr(p: fastfile_t5::Ptr) -> Ptr {
    Ptr {
        block: p.block,
        offset: p.offset,
    }
}

fn i32_at_t5(stream: &fastfile_t5::ZoneStream<'_>, body: fastfile_t5::Ptr, off: usize) -> i32 {
    stream.i32_at(body, off).unwrap_or(0)
}

fn u8_at_t5(stream: &fastfile_t5::ZoneStream<'_>, body: fastfile_t5::Ptr, off: usize) -> u8 {
    stream.u8_at(body, off).unwrap_or(0)
}

fn f32_at_t5(stream: &fastfile_t5::ZoneStream<'_>, body: fastfile_t5::Ptr, off: usize) -> f32 {
    stream.f32_at(body, off).unwrap_or(0.0)
}

fn read_bounce_array_t5(
    stream: &fastfile_t5::ZoneStream<'_>,
    body: fastfile_t5::Ptr,
    off: usize,
) -> Option<[f32; 31]> {
    let arr = match stream.ptr_at(body, off) {
        Ok(fastfile_t5::ZonePtr::Offset(q)) => stream.resolve_alias(q),
        _ => return None,
    };
    let mut values = [0.0; 31];
    for (i, value) in values.iter_mut().enumerate() {
        *value = stream.f32_at(arr, i * 4).ok()?;
    }
    Some(values)
}

fn leftover_t5_offhand_class(raw: i32) -> i32 {
    match raw {
        4 => 5,
        other => other,
    }
}

fn leftover_t5_ads_rate(trans_ms: i32, stored: f32) -> f32 {
    if stored > 0.0 {
        stored
    } else if trans_ms > 0 {
        1.0 / trans_ms as f32
    } else {
        0.0
    }
}

fn leftover_t5_cstr(
    stream: &fastfile_t5::ZoneStream<'_>,
    body: fastfile_t5::Ptr,
    off: usize,
) -> Option<String> {
    let name_ptr = match stream.ptr_at(body, off).ok()? {
        fastfile_t5::ZonePtr::Offset(q) => stream.resolve_alias(q),
        _ => return None,
    };
    stream
        .cstr(name_ptr)
        .ok()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn leftover_t5_asset_slot(
    stream: &fastfile_t5::ZoneStream<'_>,
    body: Option<fastfile_t5::Ptr>,
    off: usize,
) -> Option<Ptr> {
    let body = body?;
    match stream.ptr_at(body, off).ok()? {
        fastfile_t5::ZonePtr::Null => None,
        _ => Some(t5_to_iw4_ptr(body.at(off))),
    }
}

fn leftover_t5_reticle(
    stream: &fastfile_t5::ZoneStream<'_>,
    weap_def: Option<fastfile_t5::Ptr>,
) -> WeaponReticleAssets {
    use fastfile_t5::size as sz;
    WeaponReticleAssets {
        center_material: leftover_t5_material_name_opt(
            stream,
            weap_def,
            sz::WEAPON_DEF_RETICLE_CENTER_OFF,
        ),
        side_material: leftover_t5_material_name_opt(
            stream,
            weap_def,
            sz::WEAPON_DEF_RETICLE_SIDE_OFF,
        ),
        center_authored: leftover_t5_asset_slot(
            stream,
            weap_def,
            sz::WEAPON_DEF_RETICLE_CENTER_OFF,
        )
        .is_some(),
        side_authored: leftover_t5_asset_slot(stream, weap_def, sz::WEAPON_DEF_RETICLE_SIDE_OFF)
            .is_some(),
        center_size: weap_def
            .map(|body| i32_at_t5(stream, body, sz::WEAPON_DEF_RETICLE_CENTER_SIZE_OFF))
            .unwrap_or(0),
        side_size: weap_def
            .map(|body| i32_at_t5(stream, body, sz::WEAPON_DEF_RETICLE_SIDE_SIZE_OFF))
            .unwrap_or(0),
        ..WeaponReticleAssets::default()
    }
}

fn leftover_t5_header_name(
    stream: &fastfile_t5::ZoneStream<'_>,
    body: fastfile_t5::Ptr,
    off: usize,
) -> Option<String> {
    leftover_t5_material_name(stream, body, off)
}

fn leftover_t5_material_name(
    stream: &fastfile_t5::ZoneStream<'_>,
    body: fastfile_t5::Ptr,
    off: usize,
) -> Option<String> {
    let mat = match stream.ptr_at(body, off).ok()? {
        fastfile_t5::ZonePtr::Offset(q) => stream.resolve_alias(q),
        _ => return None,
    };
    leftover_t5_cstr(stream, mat, 0)
}

fn leftover_t5_material_name_opt(
    stream: &fastfile_t5::ZoneStream<'_>,
    body: Option<fastfile_t5::Ptr>,
    off: usize,
) -> Option<String> {
    leftover_t5_material_name(stream, body?, off)
}

fn leftover_t5_overlay_name(
    stream: &fastfile_t5::ZoneStream<'_>,
    geometry: &fastfile_t5::WeaponGeometry,
) -> Option<String> {
    leftover_t5_overlay_pick(stream, geometry).and_then(|(name, _)| name)
}

fn leftover_t5_overlay_slot(
    stream: &fastfile_t5::ZoneStream<'_>,
    geometry: &fastfile_t5::WeaponGeometry,
) -> Option<Ptr> {
    leftover_t5_overlay_pick(stream, geometry).and_then(|(_, slot)| slot)
}

fn leftover_t5_overlay_pick(
    stream: &fastfile_t5::ZoneStream<'_>,
    geometry: &fastfile_t5::WeaponGeometry,
) -> Option<(Option<String>, Option<Ptr>)> {
    let variant = geometry.variant?;
    use fastfile_t5::size as sz;
    let hi = leftover_t5_material_name(stream, variant, sz::WEAPON_VARIANT_OVERLAY_SHADER_OFF);
    let lo = leftover_t5_material_name(
        stream,
        variant,
        sz::WEAPON_VARIANT_OVERLAY_SHADER_LOWRES_OFF,
    );
    let hi_slot =
        leftover_t5_asset_slot(stream, Some(variant), sz::WEAPON_VARIANT_OVERLAY_SHADER_OFF);
    let lo_slot = leftover_t5_asset_slot(
        stream,
        Some(variant),
        sz::WEAPON_VARIANT_OVERLAY_SHADER_LOWRES_OFF,
    );
    if hi.as_deref().is_some_and(overlay_name_is_hud_iris) {
        return Some((hi, hi_slot));
    }
    if lo.as_deref().is_some_and(overlay_name_is_hud_iris) {
        return Some((lo, lo_slot));
    }
    Some((hi.or(lo), hi_slot.or(lo_slot)))
}

fn leftover_t5_zoom_fov(stream: &fastfile_t5::ZoneStream<'_>, variant: fastfile_t5::Ptr) -> f32 {
    use fastfile_t5::size as sz;
    leftover_t5_first_positive_fov(
        f32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_ZOOM_FOV1_OFF),
        f32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_ZOOM_FOV2_OFF),
        f32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_ZOOM_FOV3_OFF),
    )
}

fn leftover_t5_first_positive_fov(fov1: f32, fov2: f32, fov3: f32) -> f32 {
    for fov in [fov1, fov2, fov3] {
        if fov > 0.0 {
            return fov;
        }
    }
    0.0
}

fn leftover_t5_combat_fx(
    stream: &fastfile_t5::ZoneStream<'_>,
    geometry: &fastfile_t5::WeaponGeometry,
) -> (WeaponCombatFx, CombatFxSlots) {
    use fastfile_t5::size as sz;
    let body = geometry.weap_def;
    let slots = CombatFxSlots {
        view_flash: leftover_t5_asset_slot(stream, body, sz::WEAPON_DEF_VIEW_FLASH_OFF),
        world_flash: leftover_t5_asset_slot(stream, body, sz::WEAPON_DEF_WORLD_FLASH_OFF),
        view_shell_eject: leftover_t5_asset_slot(stream, body, sz::WEAPON_DEF_VIEW_SHELL_EJECT_OFF),
        world_shell_eject: leftover_t5_asset_slot(
            stream,
            body,
            sz::WEAPON_DEF_WORLD_SHELL_EJECT_OFF,
        ),
        view_last_shot_eject: leftover_t5_asset_slot(
            stream,
            body,
            sz::WEAPON_DEF_VIEW_LAST_SHOT_EJECT_OFF,
        ),
        world_last_shot_eject: leftover_t5_asset_slot(
            stream,
            body,
            sz::WEAPON_DEF_WORLD_LAST_SHOT_EJECT_OFF,
        ),
        explosion: None,
        tracer: None,
    };
    let fx = WeaponCombatFx {
        view_flash_hint: body
            .and_then(|b| leftover_t5_header_name(stream, b, sz::WEAPON_DEF_VIEW_FLASH_OFF)),
        world_flash_hint: body
            .and_then(|b| leftover_t5_header_name(stream, b, sz::WEAPON_DEF_WORLD_FLASH_OFF)),
        view_shell_eject_hint: body
            .and_then(|b| leftover_t5_header_name(stream, b, sz::WEAPON_DEF_VIEW_SHELL_EJECT_OFF)),
        world_shell_eject_hint: body
            .and_then(|b| leftover_t5_header_name(stream, b, sz::WEAPON_DEF_WORLD_SHELL_EJECT_OFF)),
        view_last_shot_eject_hint: body.and_then(|b| {
            leftover_t5_header_name(stream, b, sz::WEAPON_DEF_VIEW_LAST_SHOT_EJECT_OFF)
        }),
        world_last_shot_eject_hint: body.and_then(|b| {
            leftover_t5_header_name(stream, b, sz::WEAPON_DEF_WORLD_LAST_SHOT_EJECT_OFF)
        }),
        last_shot_eject_pair_authored: slots.last_shot_pair_authored(),
        ..WeaponCombatFx::default()
    };
    (fx, slots)
}

fn leftover_t5_sounds(
    stream: &fastfile_t5::ZoneStream<'_>,
    strings: &fastfile_t5::ScriptStrings,
    geometry: &fastfile_t5::WeaponGeometry,
) -> WeaponSoundAliases {
    use fastfile_t5::size as sz;
    let Some(body) = geometry.weap_def else {
        return WeaponSoundAliases::default();
    };
    WeaponSoundAliases {
        notetrack_convention: NotetrackConvention::InlinePrefix,
        fire: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_FIRE_OFF),
        fire_player: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_FIRE_PLAYER_OFF),
        empty_fire: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_EMPTY_FIRE_OFF),
        empty_fire_player: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_EMPTY_FIRE_PLAYER_OFF),
        rechamber: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RECHAMBER_OFF),
        rechamber_player: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RECHAMBER_PLAYER_OFF),
        reload: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RELOAD_OFF),
        reload_player: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RELOAD_PLAYER_OFF),
        reload_empty: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RELOAD_EMPTY_OFF),
        reload_empty_player: leftover_t5_cstr(
            stream,
            body,
            sz::WEAPON_DEF_SND_RELOAD_EMPTY_PLAYER_OFF,
        ),
        reload_start: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RELOAD_START_OFF),
        reload_start_player: leftover_t5_cstr(
            stream,
            body,
            sz::WEAPON_DEF_SND_RELOAD_START_PLAYER_OFF,
        ),
        reload_end: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RELOAD_END_OFF),
        reload_end_player: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RELOAD_END_PLAYER_OFF),
        raise_player: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_RAISE_PLAYER_OFF),
        putaway_player: leftover_t5_cstr(stream, body, sz::WEAPON_DEF_SND_PUTAWAY_PLAYER_OFF),
        notetrack_sound_map: leftover_t5_script_string_map(
            stream,
            strings,
            body,
            sz::WEAPON_DEF_NOTE_SOUND_KEYS_OFF,
            sz::WEAPON_DEF_NOTE_SOUND_VALUES_OFF,
        ),
        ..WeaponSoundAliases::default()
    }
}

fn leftover_t5_script_string_map(
    stream: &fastfile_t5::ZoneStream<'_>,
    strings: &fastfile_t5::ScriptStrings,
    body: fastfile_t5::Ptr,
    keys_off: usize,
    values_off: usize,
) -> Vec<(String, String)> {
    let keys = match stream.ptr_at(body, keys_off) {
        Ok(fastfile_t5::ZonePtr::Offset(q)) => stream.resolve_alias(q),
        _ => return Vec::new(),
    };
    let values = match stream.ptr_at(body, values_off) {
        Ok(fastfile_t5::ZonePtr::Offset(q)) => stream.resolve_alias(q),
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for i in 0..fastfile_t5::size::WEAPON_NOTETRACK_COUNT {
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
        let Some(val) = strings.get(stream, val_id).filter(|s| !s.is_empty()) else {
            continue;
        };
        out.push((key.to_owned(), val.to_owned()));
    }
    out
}

fn capture_t5_body_facts(
    stream: &fastfile_t5::ZoneStream<'_>,
    geometry: &fastfile_t5::WeaponGeometry,
) -> WeaponBodyFacts {
    use fastfile_t5::size as sz;
    let mut facts = WeaponBodyFacts {
        body_resolved: geometry.weap_def.is_some(),
        fire_time_ms: geometry.fire_time_ms,
        clip_size: geometry.clip_size,
        weap_type: geometry.weap_type,
        weap_class: remap_t5_weap_class(geometry.weap_class),
        fire_type: geometry.fire_type,
        move_speed_scale: geometry.move_speed_scale,
        ads_move_speed_scale: geometry.ads_move_speed_scale,
        rechamber_time_ms: geometry.rechamber_time_ms,
        drop_time_ms: geometry.drop_time_ms,
        raise_time_ms: geometry.raise_time_ms,
        bolt_action: geometry.bolt_action,
        select_requires_ammo_at_0x667: Some(leftover_t5_select_requires_ammo()),
        ..WeaponBodyFacts::default()
    };
    if let Some(body) = geometry.weap_def {
        facts.kill_icon_ratio = i32_at_t5(stream, body, sz::WEAPON_DEF_KILL_ICON_RATIO_OFF);
        facts.flip_kill_icon = u8_at_t5(stream, body, sz::WEAPON_DEF_FLIP_KILL_ICON_OFF) != 0;
    }
    if let Some(variant) = geometry.variant {
        facts.reload_time_ms = i32_at_t5(stream, variant, sz::WEAPON_VARIANT_RELOAD_TIME_OFF);
        facts.reload_empty_time_ms =
            i32_at_t5(stream, variant, sz::WEAPON_VARIANT_RELOAD_EMPTY_TIME_OFF);
        let ads_in_ms = i32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_TRANS_IN_OFF);
        let ads_out_ms = i32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_TRANS_OUT_OFF);
        let stored_in = f32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_IN_RATE_OFF);
        let stored_out = f32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_OUT_RATE_OFF);
        facts.ads_in_rate = leftover_t5_ads_rate(ads_in_ms, stored_in);
        facts.ads_out_rate = leftover_t5_ads_rate(ads_out_ms, stored_out);
        facts.ads_zoom_fov = leftover_t5_zoom_fov(stream, variant);
        facts.ads_zoom_in_frac =
            f32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_ZOOM_IN_FRAC_OFF);
        facts.ads_zoom_out_frac =
            f32_at_t5(stream, variant, sz::WEAPON_VARIANT_ADS_ZOOM_OUT_FRAC_OFF);
    }
    let Some(body) = geometry.weap_def else {
        return facts;
    };
    facts.impact_type = i32_at_t5(stream, body, sz::WEAPON_DEF_IMPACT_TYPE_OFF);
    facts.ammo_counter_clip = i32_at_t5(stream, body, sz::WEAPON_DEF_AMMO_COUNTER_CLIP_OFF);
    facts.start_ammo = i32_at_t5(stream, body, sz::WEAPON_DEF_START_AMMO_OFF);
    facts.max_ammo = i32_at_t5(stream, body, sz::WEAPON_DEF_MAX_AMMO_OFF);
    facts.ammo_count_clip_relative =
        u8_at_t5(stream, body, sz::WEAPON_DEF_AMMO_COUNT_CLIP_RELATIVE_OFF) != 0;
    facts.shots_per_fire = i32_at_t5(stream, body, sz::WEAPON_DEF_SHOT_COUNT_OFF);
    apply_leftover_hip_spread(
        &mut facts,
        leftover_hip_spread_block(
            |off| f32_at_t5(stream, body, off),
            sz::WEAPON_DEF_HIP_SPREAD_STAND_MIN_OFF,
        ),
    );
    facts.damage = i32_at_t5(stream, body, sz::WEAPON_DEF_DAMAGE_OFF);
    facts.min_damage = i32_at_t5(stream, body, sz::WEAPON_DEF_MIN_DAMAGE_OFF);
    facts.max_damage_range = f32_at_t5(stream, body, sz::WEAPON_DEF_MAX_DAMAGE_RANGE_OFF);
    facts.min_damage_range = f32_at_t5(stream, body, sz::WEAPON_DEF_MIN_DAMAGE_RANGE_OFF);
    facts.explosion_radius = i32_at_t5(stream, body, sz::WEAPON_DEF_EXPLOSION_RADIUS_OFF);
    facts.explosion_radius_min = i32_at_t5(stream, body, sz::WEAPON_DEF_EXPLOSION_RADIUS_MIN_OFF);
    facts.explosion_inner_damage =
        i32_at_t5(stream, body, sz::WEAPON_DEF_EXPLOSION_INNER_DAMAGE_OFF);
    facts.explosion_outer_damage =
        i32_at_t5(stream, body, sz::WEAPON_DEF_EXPLOSION_OUTER_DAMAGE_OFF);
    facts.projectile_speed = i32_at_t5(stream, body, sz::WEAPON_DEF_PROJECTILE_SPEED_OFF);
    facts.projectile_speed_up = i32_at_t5(stream, body, sz::WEAPON_DEF_PROJECTILE_SPEED_UP_OFF);
    facts.projectile_activate_dist =
        i32_at_t5(stream, body, sz::WEAPON_DEF_PROJECTILE_ACTIVATE_DIST_OFF);
    facts.projectile_explosion_type =
        i32_at_t5(stream, body, sz::WEAPON_DEF_PROJ_EXPLOSION_TYPE_OFF);
    facts.proj_impact_explode = u8_at_t5(stream, body, sz::WEAPON_DEF_PROJ_IMPACT_EXPLODE_OFF) != 0;
    facts.offhand_class =
        leftover_t5_offhand_class(i32_at_t5(stream, body, sz::WEAPON_DEF_OFFHAND_CLASS_OFF));
    facts.hold_fire_time_ms = i32_at_t5(stream, body, sz::WEAPON_DEF_HOLD_FIRE_TIME_OFF);
    facts.fuse_time_ms = i32_at_t5(stream, body, sz::WEAPON_DEF_FUSE_TIME_OFF);
    facts.cook_off_hold = u8_at_t5(stream, body, sz::WEAPON_DEF_COOK_OFF_HOLD_OFF) != 0;
    facts.offhand_hold_is_cancelable_at_0x681 =
        Some(u8_at_t5(stream, body, sz::WEAPON_DEF_OFFHAND_HOLD_IS_CANCELABLE_OFF) != 0);
    facts.parallel_bounce = read_bounce_array_t5(stream, body, sz::WEAPON_DEF_PARALLEL_BOUNCE_OFF);
    facts.perpendicular_bounce =
        read_bounce_array_t5(stream, body, sz::WEAPON_DEF_PERPENDICULAR_BOUNCE_OFF);
    facts.fire_delay_ms = i32_at_t5(stream, body, sz::WEAPON_DEF_FIRE_DELAY_OFF);
    facts.quick_drop_time_ms = i32_at_t5(stream, body, sz::WEAPON_QUICK_DROP_TIME_OFF);
    facts.quick_raise_time_ms = i32_at_t5(stream, body, sz::WEAPON_QUICK_RAISE_TIME_OFF);
    facts.rechamber_bolt_delay_ms = i32_at_t5(stream, body, sz::WEAPON_DEF_RECHAMBER_BOLT_TIME_OFF);

    facts.reload_show_rocket_time_ms = i32_at_t5(stream, body, 0x3d4);
    facts.reload_add_time_ms = i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_ADD_TIME_OFF);
    facts.reload_empty_add_time_ms =
        i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_EMPTY_ADD_TIME_OFF);
    facts.reload_start_time_ms = i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_START_TIME_OFF);
    facts.reload_start_add_time_ms =
        i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_START_ADD_TIME_OFF);
    facts.reload_end_time_ms = i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_END_TIME_OFF);
    if let Some(variant) = geometry.variant
        && u8_at_t5(stream, variant, sz::WEAPON_VARIANT_DUAL_MAG_OFF) != 0
    {
        facts.dual_mag = Some(weapon_iw4::DualMagTimes {
            reload_ms: i32_at_t5(stream, variant, sz::WEAPON_VARIANT_RELOAD_QUICK_TIME_OFF),
            reload_empty_ms: i32_at_t5(
                stream,
                variant,
                sz::WEAPON_VARIANT_RELOAD_QUICK_EMPTY_TIME_OFF,
            ),
            add_ms: i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_QUICK_ADD_TIME_OFF),
            empty_add_ms: i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_QUICK_EMPTY_ADD_TIME_OFF),
        });
    }
    facts.reload_ammo_add = i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_AMMO_ADD_OFF);
    facts.reload_start_add = i32_at_t5(stream, body, sz::WEAPON_DEF_RELOAD_START_ADD_OFF);
    facts.overlay_reticle = i32_at_t5(stream, body, sz::WEAPON_DEF_ADS_OVERLAY_RETICLE_OFF);
    facts.overlay_interface = i32_at_t5(stream, body, sz::WEAPON_DEF_ADS_OVERLAY_INTERFACE_OFF);
    facts.ads_overlay_width = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_OVERLAY_WIDTH_OFF);
    facts.ads_overlay_height = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_OVERLAY_HEIGHT_OFF);
    facts.i_reticle_side_size = i32_at_t5(stream, body, sz::WEAPON_DEF_RETICLE_SIDE_SIZE_OFF);
    facts.i_reticle_min_ofs = i32_at_t5(stream, body, sz::WEAPON_DEF_RETICLE_MIN_OFS_OFF);
    facts.hip_reticle_side_pos = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_RETICLE_SIDE_POS_OFF);
    facts.no_ads_when_mag_empty =
        u8_at_t5(stream, body, sz::WEAPON_DEF_NO_ADS_WHEN_MAG_EMPTY_OFF) != 0;
    facts.aim_down_sight = u8_at_t5(stream, body, sz::WEAPON_DEF_AIM_DOWN_SIGHT_OFF) != 0;
    facts.rechamber_while_ads = u8_at_t5(stream, body, sz::WEAPON_DEF_RECHAMBER_WHILE_ADS_OFF) != 0;
    facts.ads_fire_only = u8_at_t5(stream, body, sz::WEAPON_DEF_ADS_FIRE_ONLY_OFF) != 0;
    facts.no_partial_reload = u8_at_t5(stream, body, sz::WEAPON_DEF_NO_PARTIAL_RELOAD_OFF) != 0;
    facts.segmented_reload = u8_at_t5(stream, body, sz::WEAPON_DEF_SEGMENTED_RELOAD_OFF) != 0;
    facts.idle = WeaponIdleInputs {
        ads_idle_amount_at_0x36c: f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_IDLE_AMOUNT_OFF),
        hip_idle_amount_at_0x370: f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_IDLE_AMOUNT_OFF),
        ads_idle_speed_at_0x374: f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_IDLE_SPEED_OFF),
        hip_idle_speed_at_0x378: f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_IDLE_SPEED_OFF),
        idle_crouch_factor_at_0x37c: f32_at_t5(stream, body, sz::WEAPON_DEF_IDLE_CROUCH_FACTOR_OFF),
        idle_prone_factor_at_0x380: f32_at_t5(stream, body, sz::WEAPON_DEF_IDLE_PRONE_FACTOR_OFF),
    };
    facts.inherits_perks = leftover_t5_inherits_host_perks();
    facts.kick = leftover_t5_kick(stream, geometry);
    facts
}

fn leftover_t5_inherits_host_perks() -> bool {
    true
}

fn leftover_t5_select_requires_ammo() -> bool {
    false
}

fn leftover_t5_kick(
    stream: &fastfile_t5::ZoneStream<'_>,
    geometry: &fastfile_t5::WeaponGeometry,
) -> WeaponKickFacts {
    use fastfile_t5::size as sz;
    let mut k = WeaponKickFacts::default();
    if let Some(variant) = geometry.variant {
        k.f_ads_view_kick_center_speed = f32_at_t5(
            stream,
            variant,
            sz::WEAPON_VARIANT_ADS_VIEW_KICK_CENTER_SPEED_OFF,
        );
        k.f_hip_view_kick_center_speed = f32_at_t5(
            stream,
            variant,
            sz::WEAPON_VARIANT_HIP_VIEW_KICK_CENTER_SPEED_OFF,
        );
    }
    let Some(body) = geometry.weap_def else {
        return k;
    };
    k.gun_max_pitch = f32_at_t5(stream, body, sz::WEAPON_DEF_GUN_MAX_PITCH_OFF);
    k.gun_max_yaw = f32_at_t5(stream, body, sz::WEAPON_DEF_GUN_MAX_YAW_OFF);
    k.ads_gun_kick_reduced_kick_bullets = i32_at_t5(
        stream,
        body,
        sz::WEAPON_DEF_ADS_GUN_KICK_REDUCED_BULLETS_OFF,
    );
    k.ads_gun_kick_reduced_kick_percent = f32_at_t5(
        stream,
        body,
        sz::WEAPON_DEF_ADS_GUN_KICK_REDUCED_PERCENT_OFF,
    );
    k.ads_gun_kick_pitch_min = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_PITCH_MIN_OFF);
    k.ads_gun_kick_pitch_max = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_PITCH_MAX_OFF);
    k.ads_gun_kick_yaw_min = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_YAW_MIN_OFF);
    k.ads_gun_kick_yaw_max = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_YAW_MAX_OFF);
    k.ads_gun_kick_accel = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_ACCEL_OFF);
    k.ads_gun_kick_speed_max = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_SPEED_MAX_OFF);
    k.ads_gun_kick_speed_decay =
        f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_SPEED_DECAY_OFF);
    k.ads_gun_kick_static_decay =
        f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_GUN_KICK_STATIC_DECAY_OFF);
    k.ads_view_kick_pitch_min = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_VIEW_KICK_PITCH_MIN_OFF);
    k.ads_view_kick_pitch_max = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_VIEW_KICK_PITCH_MAX_OFF);
    k.ads_view_kick_yaw_min = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_VIEW_KICK_YAW_MIN_OFF);
    k.ads_view_kick_yaw_max = f32_at_t5(stream, body, sz::WEAPON_DEF_ADS_VIEW_KICK_YAW_MAX_OFF);
    k.hip_gun_kick_reduced_kick_bullets = i32_at_t5(
        stream,
        body,
        sz::WEAPON_DEF_HIP_GUN_KICK_REDUCED_BULLETS_OFF,
    );
    k.hip_gun_kick_reduced_kick_percent = f32_at_t5(
        stream,
        body,
        sz::WEAPON_DEF_HIP_GUN_KICK_REDUCED_PERCENT_OFF,
    );
    k.hip_gun_kick_pitch_min = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_PITCH_MIN_OFF);
    k.hip_gun_kick_pitch_max = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_PITCH_MAX_OFF);
    k.hip_gun_kick_yaw_min = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_YAW_MIN_OFF);
    k.hip_gun_kick_yaw_max = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_YAW_MAX_OFF);
    k.hip_gun_kick_accel = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_ACCEL_OFF);
    k.hip_gun_kick_speed_max = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_SPEED_MAX_OFF);
    k.hip_gun_kick_speed_decay =
        f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_SPEED_DECAY_OFF);
    k.hip_gun_kick_static_decay =
        f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_GUN_KICK_STATIC_DECAY_OFF);
    k.hip_view_kick_pitch_min = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_VIEW_KICK_PITCH_MIN_OFF);
    k.hip_view_kick_pitch_max = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_VIEW_KICK_PITCH_MAX_OFF);
    k.hip_view_kick_yaw_min = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_VIEW_KICK_YAW_MIN_OFF);
    k.hip_view_kick_yaw_max = f32_at_t5(stream, body, sz::WEAPON_DEF_HIP_VIEW_KICK_YAW_MAX_OFF);
    k
}

fn read_hide_tags_t5(
    stream: &fastfile_t5::ZoneStream<'_>,
    strings: &fastfile_t5::ScriptStrings,
    arr: Option<fastfile_t5::Ptr>,
) -> Vec<String> {
    let Some(arr) = arr else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..fastfile_t5::size::WEAPON_HIDE_TAG_COUNT {
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
