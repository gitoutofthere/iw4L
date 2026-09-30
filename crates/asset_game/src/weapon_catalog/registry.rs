use super::*;

#[derive(Clone, Debug)]
struct WeaponRow {
    name: String,
    namespace: crate::AssetNamespace,
    facts: WeaponBodyFacts,
    weap_def: Option<(u8, u32)>,
    gun_xmodel: Option<String>,
    hand_xmodel: Option<String>,
    gun_xmodel_edge: AssetEdge<FpvMeshSpace>,
    hand_xmodel_edge: AssetEdge<FpvMeshSpace>,
    rocket_model_edge: AssetEdge<FpvMeshSpace>,
    attachment_view_model_edges: Vec<AssetEdge<FpvMeshSpace>>,
    fpv_hands: [Option<(asset_model::FpvHands, crate::FpvMeshIndex)>; 2],
    fpv_mount_plan: Option<Result<asset_model::FpvMountPlan, asset_model::FpvMountError>>,
    fpv_assemblies: [Option<Result<crate::FpvSideAssemblies, String>>; 2],
    world_model: Option<String>,
    world_model_edge: AssetEdge<WorldWeaponSpace>,
    attachment_world_model_edges: Vec<AssetEdge<WorldWeaponSpace>>,
    attachment_world_mounts: Vec<Option<String>>,
    projectile_model: Option<String>,
    projectile_model_edge: AssetEdge<ProjectileModelSpace>,
    rocket_model: Option<String>,
    sz_xanims: [Option<String>; WEAPON_ANIM_SLOTS],
    sz_xanim_edges: [AssetEdge<XAnimSpace>; WEAPON_ANIM_SLOTS],
    sz_xanim_right_edges: [AssetEdge<XAnimSpace>; WEAPON_ANIM_SLOTS],
    sz_xanim_left_edges: [AssetEdge<XAnimSpace>; WEAPON_ANIM_SLOTS],
    notetrack_actions: HashMap<String, LinkedNotetrackAction>,
    sz_xanims_right: [Option<String>; WEAPON_ANIM_SLOTS],
    sz_xanims_left: [Option<String>; WEAPON_ANIM_SLOTS],
    hide_tags: Vec<String>,
    attachment_view_models: Vec<String>,
    attachment_world_models: Vec<String>,
    iw5_configuration: Option<(u32, Iw5AttachmentSelection)>,
    prepared_attachments: Vec<String>,
    iw5_attachment_slots: [Option<String>; fastfile_iw5::size::WEAPON_ATTACHMENT_SLOT_COUNT],
    iw5_reload_overrides: Vec<fastfile_iw5::ReloadOverride>,
    iw5_anim_overrides: Vec<LeftoverAnimOverride>,
    iw5_fx_overrides: Vec<Iw5FxOverride>,
    iw5_notetrack_overrides: Vec<Iw5NotetrackOverride>,
    sounds: WeaponSoundAliases,
    combat_fx: WeaponCombatFx,
    reticle: WeaponReticleAssets,
    hud_material_edges: WeaponHudMaterialEdges,
    overlay_material: Option<String>,
    overlay_image: Option<String>,
    overlay_material_from_slot: bool,
    hud_icon: Option<String>,
    hud_icon_from_slot: bool,
    pickup_icon: Option<String>,
    pickup_icon_image: Option<String>,
    pickup_icon_authored: bool,
    pickup_icon_ratio: i32,
    hud_icon_ratio: i32,
    hud_icon_image: Option<String>,
    dpad_icon_image: Option<String>,
    dpad_icon_ratio: i32,
    kill_icon: Option<String>,
    kill_icon_from_slot: bool,
    kill_icon_image: Option<String>,
    projectile_fx: WeaponProjectileFx,
    proj_trail: Option<String>,
    proj_trail_from_slot: bool,
    proj_beacon: Option<String>,
    proj_beacon_from_slot: bool,
    proj_ignition: Option<String>,
    proj_ignition_from_slot: bool,
    display_name_key: Option<String>,
}

impl Default for WeaponRow {
    fn default() -> Self {
        Self {
            name: String::new(),
            namespace: crate::AssetNamespace::Iw4,
            facts: WeaponBodyFacts::default(),
            weap_def: None,
            gun_xmodel: None,
            hand_xmodel: None,
            gun_xmodel_edge: AssetEdge::Absent,
            hand_xmodel_edge: AssetEdge::Absent,
            rocket_model_edge: AssetEdge::Absent,
            attachment_view_model_edges: Vec::new(),
            fpv_hands: [None, None],
            fpv_mount_plan: None,
            fpv_assemblies: [None, None],
            world_model: None,
            world_model_edge: AssetEdge::Absent,
            attachment_world_model_edges: Vec::new(),
            attachment_world_mounts: Vec::new(),
            projectile_model: None,
            projectile_model_edge: AssetEdge::Absent,
            rocket_model: None,
            sz_xanims: [const { None }; WEAPON_ANIM_SLOTS],
            sz_xanim_edges: [AssetEdge::Absent; WEAPON_ANIM_SLOTS],
            sz_xanim_right_edges: [AssetEdge::Absent; WEAPON_ANIM_SLOTS],
            sz_xanim_left_edges: [AssetEdge::Absent; WEAPON_ANIM_SLOTS],
            notetrack_actions: HashMap::new(),
            sz_xanims_right: [const { None }; WEAPON_ANIM_SLOTS],
            sz_xanims_left: [const { None }; WEAPON_ANIM_SLOTS],
            hide_tags: Vec::new(),
            attachment_view_models: Vec::new(),
            attachment_world_models: Vec::new(),
            iw5_configuration: None,
            prepared_attachments: Vec::new(),
            iw5_attachment_slots: std::array::from_fn(|_| None),
            iw5_reload_overrides: Vec::new(),
            iw5_anim_overrides: Vec::new(),
            iw5_fx_overrides: Vec::new(),
            iw5_notetrack_overrides: Vec::new(),
            sounds: WeaponSoundAliases::default(),
            combat_fx: WeaponCombatFx::default(),
            reticle: WeaponReticleAssets::default(),
            hud_material_edges: WeaponHudMaterialEdges::default(),
            overlay_material: None,
            overlay_image: None,
            overlay_material_from_slot: false,
            hud_icon: None,
            hud_icon_from_slot: false,
            pickup_icon: None,
            pickup_icon_image: None,
            pickup_icon_authored: false,
            pickup_icon_ratio: 0,
            hud_icon_ratio: 0,
            hud_icon_image: None,
            dpad_icon_image: None,
            dpad_icon_ratio: 0,
            kill_icon: None,
            kill_icon_from_slot: false,
            kill_icon_image: None,
            projectile_fx: WeaponProjectileFx::default(),
            proj_trail: None,
            proj_trail_from_slot: false,
            proj_beacon: None,
            proj_beacon_from_slot: false,
            proj_ignition: None,
            proj_ignition_from_slot: false,
            display_name_key: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WeaponRegistry {
    rows: Vec<WeaponRow>,
    world_catalog_identity: u64,
    iw5_attachments: HashMap<String, Iw5ScopeRow>,
    iw5_candidates: Vec<Iw5ConfigurationCandidate>,
    configurations: HashMap<crate::WeaponSelection, u32>,
    by_name: HashMap<String, u32>,
    by_namespaced: HashMap<(crate::AssetNamespace, String), u32>,
    item_groups: HashMap<(crate::AssetNamespace, String), String>,
    families: crate::WeaponFamilies,
    fpv_clip_tracks: Arc<crate::FpvClipTracks>,
    revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownWeaponName {
    pub name: String,
}

impl core::fmt::Display for UnknownWeaponName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unknown weapon name `{}`", self.name)
    }
}

impl std::error::Error for UnknownWeaponName {}

#[derive(Clone, Debug, Default)]
pub struct Iw5PreparationCensus {
    pub prepared: usize,
    pub refused: Vec<crate::WeaponSelection>,
}

#[derive(Clone, Debug, Default)]
pub struct WeaponBuild {
    registry: WeaponRegistry,
    combat_slots: Vec<CombatFxSlots>,
    family_tables: Vec<(crate::AssetNamespace, crate::CapturedStringTable)>,
}

impl std::ops::Deref for WeaponBuild {
    type Target = WeaponRegistry;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}

impl WeaponBuild {
    pub fn publish(self) -> WeaponRegistry {
        let mut registry = self.registry;
        registry.families = crate::WeaponFamilies::build(&self.family_tables, &registry);
        registry.build_iw5_candidates();
        registry
    }

    pub fn set_family_tables(
        &mut self,
        tables: Vec<(crate::AssetNamespace, crate::CapturedStringTable)>,
    ) {
        self.family_tables = tables;
    }

    pub fn prepare_iw5_configurations(&mut self) -> Iw5PreparationCensus {
        let families = crate::WeaponFamilies::build(&self.family_tables, &self.registry);
        let mut census = Iw5PreparationCensus::default();
        let mut prepared = Vec::new();
        for (base_id, selection) in families.iw5_candidate_selections() {
            if selection.attachments.is_empty() {
                continue;
            }
            let Some(namespace) = selection.family.as_ref().map(|key| key.namespace) else {
                continue;
            };
            let attachments = families.normalize(namespace, &selection.attachments);
            let selection = crate::WeaponSelection {
                attachments,
                ..selection
            };
            let row = self
                .registry
                .resolve_iw5_attachment_slots(base_id, &selection.attachments)
                .ok()
                .and_then(|native| self.registry.compose_iw5_configuration(base_id, native));
            match row {
                Some(row) => prepared.push((base_id, selection, row)),
                None => census.refused.push(selection),
            }
        }
        census.prepared = prepared.len();
        for (base_id, selection, mut row) in prepared {
            row.prepared_attachments = selection.attachments.clone();
            self.registry
                .configurations
                .insert(selection, self.registry.rows.len() as u32);
            let slots = self
                .combat_slots
                .get(base_id as usize)
                .copied()
                .unwrap_or_default();
            self.combat_slots
                .resize(self.registry.rows.len(), CombatFxSlots::default());
            self.combat_slots.push(slots);
            self.registry.rows.push(row);
        }
        self.registry.rebuild_name_maps();
        self.registry.revision = mint_weapon_revision();
        census
    }

    pub fn absorb(&mut self, other: Self) {
        if other.registry.is_empty() {
            self.registry
                .iw5_attachments
                .extend(other.registry.iw5_attachments);
            self.family_tables.extend(other.family_tables);
            return;
        }
        if self.registry.rows.is_empty() {
            *self = other;
            return;
        }
        self.registry
            .rows
            .extend(other.registry.rows.into_iter().skip(1));
        self.registry
            .iw5_attachments
            .extend(other.registry.iw5_attachments);
        self.combat_slots
            .extend(other.combat_slots.into_iter().skip(1));
        self.family_tables.extend(other.family_tables);
        self.registry.item_groups.extend(other.registry.item_groups);
        self.registry.rebuild_name_maps();
        self.registry.revision = mint_weapon_revision();
    }

    pub fn resolve_combat_fx(&mut self, fx: &crate::FxCatalog, tracers: &crate::TracerCatalog) {
        let n = self.registry.rows.len().min(self.combat_slots.len());
        for i in 1..n {
            stamp_combat_fx(
                &mut self.registry.rows[i].combat_fx,
                self.combat_slots[i],
                fx,
                tracers,
            );
        }
    }

    pub fn stamp_namespace(&mut self, ns: crate::AssetNamespace) {
        let attachments = &self.registry.iw5_attachments;
        for row in self.registry.rows.iter_mut().skip(1) {
            row.namespace = ns;
            if ns == crate::AssetNamespace::Iw5 {
                if let Some((view, world)) =
                    iw5_default_scope_models(&row.iw5_attachment_slots, attachments)
                {
                    row.attachment_view_models.extend(view);
                    row.attachment_world_models.extend(world);
                }
            }
        }
        self.registry.rebuild_name_maps();
        self.registry.revision = mint_weapon_revision();
    }

    pub fn resolve_hud_material_edges(&mut self, materials: &crate::MaterialDefinitions) {
        for row in &mut self.registry.rows {
            if row.overlay_image.is_none()
                && let Some(name) = row.overlay_material.as_deref()
                && let Some(index) = materials.material_index_by_name(name)
                && let Some(material) = materials.materials.get(index.order())
            {
                row.overlay_image = materials.hud_image_name(material).map(str::to_owned);
            }
            row.reticle.center_edge = material_hint_edge(
                row.reticle.center_material.as_deref(),
                row.reticle.center_authored,
                materials,
            );
            row.reticle.side_edge = material_hint_edge(
                row.reticle.side_material.as_deref(),
                row.reticle.side_authored,
                materials,
            );
            row.hud_material_edges = WeaponHudMaterialEdges {
                overlay: material_hint_edge(
                    row.overlay_material.as_deref(),
                    row.overlay_material_from_slot,
                    materials,
                ),
                hud_icon: material_hint_edge(
                    row.hud_icon.as_deref(),
                    row.hud_icon_from_slot,
                    materials,
                ),
                pickup_icon: material_hint_edge(
                    row.pickup_icon.as_deref(),
                    row.pickup_icon_authored,
                    materials,
                ),
                kill_icon: material_hint_edge(
                    row.kill_icon.as_deref(),
                    row.kill_icon_from_slot,
                    materials,
                ),
            };
        }
    }

    pub fn resolve_projectile_fx_edges(&mut self, fx: &crate::FxCatalog) {
        for row in &mut self.registry.rows {
            row.projectile_fx = WeaponProjectileFx {
                trail: fx_hint_edge(row.proj_trail_from_slot, row.proj_trail.as_deref(), fx),
                beacon: fx_hint_edge(row.proj_beacon_from_slot, row.proj_beacon.as_deref(), fx),
                ignition: fx_hint_edge(
                    row.proj_ignition_from_slot,
                    row.proj_ignition.as_deref(),
                    fx,
                ),
            };
        }
    }

    pub fn resolve_sz_xanim_edges(&mut self, xanims: &crate::XAnimCatalog) {
        for row in &mut self.registry.rows {
            let mut edges = [AssetEdge::Absent; WEAPON_ANIM_SLOTS];
            for (edge, hint) in edges.iter_mut().zip(row.sz_xanims.iter()) {
                *edge = xanim_hint_edge(hint.as_deref(), row.namespace, xanims);
            }
            row.sz_xanim_edges = edges;
            row.sz_xanim_right_edges = std::array::from_fn(|slot| {
                xanim_hint_edge(row.sz_xanims_right[slot].as_deref(), row.namespace, xanims)
            });
            row.sz_xanim_left_edges = std::array::from_fn(|slot| {
                xanim_hint_edge(row.sz_xanims_left[slot].as_deref(), row.namespace, xanims)
            });
        }
    }

    pub fn resolve_notetrack_actions(&mut self, xanims: &crate::XAnimCatalog) -> (usize, usize) {
        let mut linked = 0;
        let mut inline = 0;
        for row in &mut self.registry.rows {
            let mut actions: HashMap<String, LinkedNotetrackAction> = HashMap::new();
            match row.sounds.notetrack_convention {
                NotetrackConvention::SoundMap => {
                    for (note, alias) in &row.sounds.notetrack_sound_map {
                        if !alias.is_empty() {
                            let action = actions.entry(note.to_ascii_lowercase()).or_default();
                            if action.sound_alias.is_none() {
                                action.sound_alias = Some(alias.clone());
                            }
                        }
                    }
                    for (note, alias) in &row.sounds.notetrack_rumble_map {
                        if !alias.is_empty() {
                            let action = actions.entry(note.to_ascii_lowercase()).or_default();
                            if action.rumble_alias.is_none() {
                                action.rumble_alias = Some(alias.clone());
                            }
                        }
                    }
                }
                NotetrackConvention::InlinePrefix => {
                    let indices: std::collections::BTreeSet<_> = row
                        .sz_xanim_edges
                        .iter()
                        .chain(&row.sz_xanim_right_edges)
                        .chain(&row.sz_xanim_left_edges)
                        .filter_map(|edge| edge.bound_index())
                        .collect();
                    for index in indices {
                        let Some(clip) = xanims.clip_at(index) else {
                            continue;
                        };
                        for notify in &clip.notifies {
                            let sound = t5_inline_note_alias(&notify.name, T5_NOTE_SOUND_PREFIX);
                            let rumble = t5_inline_note_alias(&notify.name, T5_NOTE_RUMBLE_PREFIX);
                            if sound.is_none() && rumble.is_none() {
                                continue;
                            }
                            let action =
                                actions.entry(notify.name.to_ascii_lowercase()).or_default();
                            action.sound_alias = sound.map(str::to_owned);
                            action.rumble_alias = rumble.map(str::to_owned);
                        }
                    }
                }
            }
            linked += actions.len();
            if row.sounds.notetrack_convention == NotetrackConvention::InlinePrefix {
                inline += actions.len();
            }
            row.notetrack_actions = actions;
        }
        (linked, inline)
    }

    pub fn resolve_fpv_mesh_edges(&mut self, fpv: &crate::FpvMeshCatalog) {
        for row in &mut self.registry.rows {
            row.gun_xmodel_edge = fpv_model_edge(row.gun_xmodel.as_deref(), row.namespace, fpv);
            row.hand_xmodel_edge = fpv_model_edge(row.hand_xmodel.as_deref(), row.namespace, fpv);
            row.rocket_model_edge = fpv_model_edge(row.rocket_model.as_deref(), row.namespace, fpv);
            row.attachment_view_model_edges = row
                .attachment_view_models
                .iter()
                .map(|name| fpv_model_edge(Some(name), row.namespace, fpv))
                .collect();
            row.fpv_mount_plan = row.gun_xmodel_edge.bound_index().and_then(|gun| {
                let attachments: Option<Vec<_>> = row
                    .attachment_view_model_edges
                    .iter()
                    .map(|edge| edge.bound_index().map(crate::FpvMeshIndex::from_order))
                    .collect();
                let rocket = if row.rocket_model_edge.is_absent() {
                    Some(None)
                } else {
                    row.rocket_model_edge
                        .bound_index()
                        .map(|index| Some(crate::FpvMeshIndex::from_order(index)))
                }?;
                Some(asset_model::plan_fpv_mounts(
                    fpv,
                    crate::FpvMeshIndex::from_order(gun),
                    &attachments?,
                    rocket,
                ))
            });
        }
    }

    pub fn resolve_fpv_hands(
        &mut self,
        fpv: &crate::FpvMeshCatalog,
        bodies: &asset_model::BodyMeshCatalog,
    ) {
        for row in &mut self.registry.rows {
            let map_ns = fpv.map_namespace.unwrap_or(row.namespace);
            let hand_name = row
                .hand_xmodel_edge
                .bound_index()
                .and_then(|index| fpv.get_at(index))
                .map(|entry| entry.skel.name.as_str());
            row.fpv_hands = std::array::from_fn(|side| {
                let kit = bodies.kits().kit(side == 1);
                let choice =
                    asset_model::FpvHands::resolve(fpv, map_ns, kit, hand_name, row.namespace);
                let (ns, name) = choice.key()?;
                let index = fpv.index_by_name(ns, name)?;
                let skel = &fpv.get_at(index)?.skel;
                if skel.pose.is_none() || !skel.bone_names.iter().any(|bone| bone == "tag_weapon") {
                    return None;
                }
                Some((choice, crate::FpvMeshIndex::from_order(index)))
            });
        }
    }

    pub fn resolve_fpv_assemblies(
        &mut self,
        fpv: &crate::FpvMeshCatalog,
        xanims: &crate::XAnimCatalog,
    ) -> FpvAssemblyCensus {
        let mut shared: HashMap<crate::FpvAssemblyKey, Result<Arc<crate::FpvAssembly>, String>> =
            HashMap::new();
        let mut tracks = crate::FpvClipTracks::default();
        let mut census = FpvAssemblyCensus::default();
        let hide_tags: Vec<Vec<String>> = (0..self.registry.rows.len() as u32)
            .map(|id| crate::effective_hide_tags(&self.registry, id))
            .collect();
        for (row, hide_tags) in self.registry.rows.iter_mut().zip(hide_tags) {
            row.fpv_assemblies = [None, None];
            let Some(Ok(mounts)) = &row.fpv_mount_plan else {
                continue;
            };
            let clips: std::collections::BTreeSet<usize> = row
                .sz_xanim_edges
                .iter()
                .chain(&row.sz_xanim_right_edges)
                .chain(&row.sz_xanim_left_edges)
                .filter_map(|edge| edge.bound_index())
                .collect();
            let mut assemble = |hands: crate::FpvMeshIndex, rocket: bool| {
                let key = crate::FpvAssemblyKey {
                    hands,
                    gun: mounts.gun,
                    attachments: mounts.attachments.iter().map(|mount| mount.model).collect(),
                    rocket: rocket
                        .then(|| mounts.rocket.as_ref().map(|mount| mount.model))
                        .flatten(),
                    hide_tags: hide_tags.clone(),
                };
                shared
                    .entry(key)
                    .or_insert_with(|| {
                        census.built += 1;
                        crate::FpvAssembly::build(fpv, hands, mounts, rocket, &hide_tags)
                            .map(Arc::new)
                            .map_err(|error| error.to_string())
                    })
                    .clone()
            };
            let sides: [Option<Result<crate::FpvSideAssemblies, String>>; 2] =
                std::array::from_fn(|side| {
                    let (_, hands) = row.fpv_hands[side].as_ref()?;
                    let bare = match assemble(*hands, false) {
                        Ok(bare) => bare,
                        Err(error) => return Some(Err(error)),
                    };
                    let rocket = match mounts.rocket.is_some().then(|| assemble(*hands, true)) {
                        None => None,
                        Some(Ok(rocket)) => Some(rocket),
                        Some(Err(error)) => return Some(Err(error)),
                    };
                    Some(Ok(crate::FpvSideAssemblies { bare, rocket }))
                });
            for side in sides.iter().flatten().flatten() {
                for assembly in std::iter::once(&side.bare).chain(&side.rocket) {
                    for &clip_index in &clips {
                        let Some(clip) = xanims.clip_at(clip_index) else {
                            continue;
                        };
                        for part in &assembly.parts {
                            tracks.bind(fpv, clip_index, &clip, part.model);
                        }
                    }
                }
            }
            for side in sides.iter().flatten() {
                match side {
                    Ok(_) => census.linked += 1,
                    Err(_) => census.refused += 1,
                }
            }
            row.fpv_assemblies = sides;
        }
        census.clip_tables = tracks.len();
        self.registry.fpv_clip_tracks = Arc::new(tracks);
        census
    }

    pub fn resolve_world_model_edges(&mut self, catalog: &crate::WorldWeaponCatalog) {
        self.registry.world_catalog_identity = catalog.identity();
        for row in &mut self.registry.rows {
            row.world_model_edge = world_model_edge(row.world_model.as_deref(), catalog);
            row.attachment_world_model_edges = row
                .attachment_world_models
                .iter()
                .map(|name| world_model_edge(Some(name), catalog))
                .collect();
            let gun = row
                .world_model_edge
                .bound_index()
                .and_then(|index| catalog.get_at(index));
            row.attachment_world_mounts = row
                .attachment_world_model_edges
                .iter()
                .map(|edge| {
                    let root = catalog
                        .get_at(edge.bound_index()?)?
                        .skel
                        .bone_names
                        .first()?;
                    gun?.skel
                        .bone_names
                        .iter()
                        .find(|bone| bone.eq_ignore_ascii_case(root))
                        .cloned()
                })
                .collect();
        }
    }

    pub fn apply_stats_item_groups(&mut self, table: &crate::CapturedStringTable) {
        for id in 1..=self.len() as u32 {
            let Some(ns) = self.namespace_of(id) else {
                continue;
            };
            let name = self.name_of(id).to_owned();
            if name.is_empty() {
                continue;
            }
            if let Some(group) = crate::item_group_for_weapon(table, &name) {
                self.registry
                    .item_groups
                    .insert((ns, name), group.to_owned());
            }
        }
    }

    pub fn apply_stats_tables<'a>(
        &mut self,
        tables: impl IntoIterator<Item = &'a crate::CapturedStringTable>,
    ) {
        for table in tables {
            if crate::is_stats_table_name(&table.name) {
                self.apply_stats_item_groups(table);
            }
        }
    }

    pub fn stamp_projectile_model_edges(
        &mut self,
        catalog: &crate::ProjectileMeshCatalog,
        zone: ZoneOwner,
    ) {
        for row in &mut self.registry.rows {
            row.projectile_model_edge = match row.projectile_model.as_deref() {
                None | Some("") => AssetEdge::Absent,
                Some(name) => match catalog.index_by_name(name) {
                    Some(order) => AssetEdge::bind_order(order, zone),
                    None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
                },
            };
        }
    }

    pub(super) fn from_catalog(
        mut entries: Vec<CatalogWeapon>,
        iw5_attachments: HashMap<String, Iw5ScopeRow>,
    ) -> Self {
        let mut gun_by_def: HashMap<(u8, u32), String> = HashMap::new();
        let mut hand_by_def: HashMap<(u8, u32), String> = HashMap::new();
        let mut world_by_def: HashMap<(u8, u32), String> = HashMap::new();
        let mut projectile_by_def: HashMap<(u8, u32), String> = HashMap::new();
        let mut rocket_by_def: HashMap<(u8, u32), String> = HashMap::new();
        let mut sounds_by_def: HashMap<(u8, u32), WeaponSoundAliases> = HashMap::new();
        let mut combat_fx_by_def: HashMap<(u8, u32), WeaponCombatFx> = HashMap::new();
        let mut combat_slots_by_def: HashMap<(u8, u32), CombatFxSlots> = HashMap::new();
        let mut right_by_def: HashMap<(u8, u32), [Option<String>; WEAPON_ANIM_SLOTS]> =
            HashMap::new();
        let mut left_by_def: HashMap<(u8, u32), [Option<String>; WEAPON_ANIM_SLOTS]> =
            HashMap::new();
        for entry in &entries {
            if let (Some(key), Some(gun)) = (entry.weap_def, entry.gun_xmodel.as_ref()) {
                gun_by_def.entry(key).or_insert_with(|| gun.clone());
            }
            if let (Some(key), Some(hand)) = (entry.weap_def, entry.hand_xmodel.as_ref()) {
                hand_by_def.entry(key).or_insert_with(|| hand.clone());
            }
            if let (Some(key), Some(world)) = (entry.weap_def, entry.world_model.as_ref()) {
                world_by_def.entry(key).or_insert_with(|| world.clone());
            }
            if let (Some(key), Some(proj)) = (entry.weap_def, entry.projectile_model.as_ref()) {
                projectile_by_def.entry(key).or_insert_with(|| proj.clone());
            }
            if let (Some(key), Some(rocket)) = (entry.weap_def, entry.rocket_model.as_ref()) {
                rocket_by_def.entry(key).or_insert_with(|| rocket.clone());
            }
            if let Some(key) = entry.weap_def {
                merge_sound_aliases(sounds_by_def.entry(key).or_default(), &entry.sounds);
                merge_combat_fx(combat_fx_by_def.entry(key).or_default(), &entry.combat_fx);
                merge_combat_slots(
                    combat_slots_by_def.entry(key).or_default(),
                    &entry.combat_slots,
                );
                if xanims_idle(&entry.sz_xanims_right).is_some() {
                    right_by_def
                        .entry(key)
                        .or_insert_with(|| entry.sz_xanims_right.clone());
                }
                if xanims_idle(&entry.sz_xanims_left).is_some() {
                    left_by_def
                        .entry(key)
                        .or_insert_with(|| entry.sz_xanims_left.clone());
                }
            }
        }
        for entry in &mut entries {
            if entry.gun_xmodel.is_none() {
                if let Some(key) = entry.weap_def {
                    entry.gun_xmodel = gun_by_def.get(&key).cloned();
                }
            }
            if entry.hand_xmodel.is_none() {
                if let Some(key) = entry.weap_def {
                    entry.hand_xmodel = hand_by_def.get(&key).cloned();
                }
            }
            if entry.world_model.is_none() {
                if let Some(key) = entry.weap_def {
                    entry.world_model = world_by_def.get(&key).cloned();
                }
            }
            if entry.projectile_model.is_none() {
                if let Some(key) = entry.weap_def {
                    entry.projectile_model = projectile_by_def.get(&key).cloned();
                }
            }
            if entry.rocket_model.is_none() {
                if let Some(key) = entry.weap_def {
                    entry.rocket_model = rocket_by_def.get(&key).cloned();
                }
            }
            if let Some(key) = entry.weap_def {
                if let Some(shared) = sounds_by_def.get(&key) {
                    merge_sound_aliases(&mut entry.sounds, shared);
                }
                if let Some(shared) = combat_fx_by_def.get(&key) {
                    merge_combat_fx(&mut entry.combat_fx, shared);
                }
                if let Some(&shared) = combat_slots_by_def.get(&key) {
                    merge_combat_slots(&mut entry.combat_slots, &shared);
                }
                if xanims_idle(&entry.sz_xanims_right).is_none() {
                    if let Some(shared) = right_by_def.get(&key) {
                        entry.sz_xanims_right = shared.clone();
                    }
                }
                if xanims_idle(&entry.sz_xanims_left).is_none() {
                    if let Some(shared) = left_by_def.get(&key) {
                        entry.sz_xanims_left = shared.clone();
                    }
                }
            }
        }

        let mut by_name: HashMap<String, CatalogWeapon> = HashMap::new();
        for entry in entries {
            let key = normalize_weapon_name(&entry.name);
            match by_name.entry(key) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(entry);
                }
                std::collections::hash_map::Entry::Occupied(mut slot) => {
                    let existing = slot.get_mut();
                    if existing.gun_xmodel.is_none() {
                        existing.gun_xmodel = entry.gun_xmodel;
                    }
                    if existing.hand_xmodel.is_none() {
                        existing.hand_xmodel = entry.hand_xmodel;
                    }
                    if existing.world_model.is_none() {
                        existing.world_model = entry.world_model;
                    }
                    if existing.projectile_model.is_none() {
                        existing.projectile_model = entry.projectile_model;
                    }
                    if existing.rocket_model.is_none() {
                        existing.rocket_model = entry.rocket_model;
                    }
                    if existing.overlay_material.is_none() {
                        existing.overlay_material = entry.overlay_material;
                        existing.overlay_image = entry.overlay_image;
                        existing.overlay_material_slot = entry.overlay_material_slot;
                    }
                    if existing.hud_icon.is_none() {
                        existing.hud_icon = entry.hud_icon;
                        existing.hud_icon_slot = entry.hud_icon_slot;
                        existing.hud_icon_image = entry.hud_icon_image;
                    }
                    if !existing.reticle.center_authored
                        && !existing.reticle.side_authored
                        && (entry.reticle.center_authored || entry.reticle.side_authored)
                    {
                        existing.reticle = entry.reticle;
                    }
                    if existing.kill_icon.is_none() {
                        existing.kill_icon = entry.kill_icon;
                        existing.kill_icon_slot = entry.kill_icon_slot;
                        existing.kill_icon_image = entry.kill_icon_image;
                    }
                    if existing.proj_trail.is_none() {
                        existing.proj_trail = entry.proj_trail;
                        existing.proj_trail_slot = entry.proj_trail_slot;
                        existing.projectile_fx.trail = entry.projectile_fx.trail;
                    }
                    if existing.proj_beacon.is_none() {
                        existing.proj_beacon = entry.proj_beacon;
                        existing.proj_beacon_slot = entry.proj_beacon_slot;
                        existing.projectile_fx.beacon = entry.projectile_fx.beacon;
                    }
                    if existing.proj_ignition.is_none() {
                        existing.proj_ignition = entry.proj_ignition;
                        existing.proj_ignition_slot = entry.proj_ignition_slot;
                        existing.projectile_fx.ignition = entry.projectile_fx.ignition;
                    }
                    merge_sz_xanims(&mut existing.sz_xanims, entry.sz_xanims);
                    merge_sz_xanims(&mut existing.sz_xanims_right, entry.sz_xanims_right);
                    merge_sz_xanims(&mut existing.sz_xanims_left, entry.sz_xanims_left);
                    if existing.hide_tags.is_empty() && !entry.hide_tags.is_empty() {
                        existing.hide_tags = entry.hide_tags;
                    }
                    for (existing_slot, new_slot) in existing
                        .iw5_attachment_slots
                        .iter_mut()
                        .zip(entry.iw5_attachment_slots)
                    {
                        if existing_slot.is_none() {
                            *existing_slot = new_slot;
                        }
                    }
                    if existing.iw5_reload_overrides.is_empty() {
                        existing.iw5_reload_overrides = entry.iw5_reload_overrides;
                    }
                    if existing.iw5_anim_overrides.is_empty() {
                        existing.iw5_anim_overrides = entry.iw5_anim_overrides;
                    }
                    if existing.iw5_fx_overrides.is_empty() {
                        existing.iw5_fx_overrides = entry.iw5_fx_overrides;
                    }
                    if existing.iw5_notetrack_overrides.is_empty() {
                        existing.iw5_notetrack_overrides = entry.iw5_notetrack_overrides;
                    }
                    merge_sound_aliases(&mut existing.sounds, &entry.sounds);
                    if existing.sounds.leftover_sound_overrides.is_empty() {
                        existing.sounds.leftover_sound_overrides =
                            entry.sounds.leftover_sound_overrides.clone();
                    }
                    merge_combat_fx(&mut existing.combat_fx, &entry.combat_fx);
                    merge_combat_slots(&mut existing.combat_slots, &entry.combat_slots);
                    merge_body_facts(&mut existing.facts, entry.facts);
                }
            }
        }
        let mut names: Vec<String> = by_name.keys().cloned().collect();
        names.sort_unstable();

        let mut rows = Vec::with_capacity(names.len() + 1);
        let mut combat_slots = Vec::with_capacity(names.len() + 1);
        let mut index_of = HashMap::with_capacity(names.len());
        rows.push(WeaponRow::default());
        combat_slots.push(CombatFxSlots::default());
        for name in names {
            let entry = by_name.remove(&name).expect("key from map");
            let id = rows.len() as u32;
            index_of.insert(name.clone(), id);
            combat_slots.push(entry.combat_slots);
            rows.push(WeaponRow {
                name,
                namespace: crate::AssetNamespace::Iw4,
                facts: entry.facts,
                weap_def: entry.weap_def,
                gun_xmodel: entry.gun_xmodel,
                hand_xmodel: entry.hand_xmodel,
                gun_xmodel_edge: AssetEdge::Absent,
                hand_xmodel_edge: AssetEdge::Absent,
                rocket_model_edge: AssetEdge::Absent,
                attachment_view_model_edges: Vec::new(),
                fpv_hands: [None, None],
                fpv_mount_plan: None,
                fpv_assemblies: [None, None],
                world_model: entry.world_model,
                world_model_edge: AssetEdge::Absent,
                attachment_world_model_edges: Vec::new(),
                attachment_world_mounts: Vec::new(),
                projectile_model: entry.projectile_model,
                projectile_model_edge: AssetEdge::Absent,
                rocket_model: entry.rocket_model,
                sz_xanims: entry.sz_xanims,
                sz_xanim_edges: [AssetEdge::Absent; WEAPON_ANIM_SLOTS],
                sz_xanim_right_edges: [AssetEdge::Absent; WEAPON_ANIM_SLOTS],
                sz_xanim_left_edges: [AssetEdge::Absent; WEAPON_ANIM_SLOTS],
                notetrack_actions: HashMap::new(),
                sz_xanims_right: entry.sz_xanims_right,
                sz_xanims_left: entry.sz_xanims_left,
                hide_tags: entry.hide_tags,
                attachment_view_models: Vec::new(),
                attachment_world_models: Vec::new(),
                iw5_configuration: None,
                prepared_attachments: Vec::new(),
                iw5_attachment_slots: entry.iw5_attachment_slots,
                iw5_reload_overrides: entry.iw5_reload_overrides,
                iw5_anim_overrides: entry.iw5_anim_overrides,
                iw5_fx_overrides: entry.iw5_fx_overrides,
                iw5_notetrack_overrides: entry.iw5_notetrack_overrides,
                sounds: entry.sounds,
                combat_fx: entry.combat_fx,
                reticle: entry.reticle,
                hud_material_edges: entry.hud_material_edges,
                overlay_material_from_slot: entry.overlay_material_slot.is_some(),
                overlay_material: entry.overlay_material,
                overlay_image: entry.overlay_image,
                hud_icon_from_slot: entry.hud_icon_slot.is_some(),
                hud_icon: entry.hud_icon,
                hud_icon_image: entry.hud_icon_image,
                pickup_icon: entry.pickup_icon,
                pickup_icon_image: entry.pickup_icon_image,
                pickup_icon_authored: entry.pickup_icon_slot.is_some(),
                pickup_icon_ratio: entry.pickup_icon_ratio,
                hud_icon_ratio: entry.hud_icon_ratio,
                kill_icon_from_slot: entry.kill_icon_slot.is_some(),
                dpad_icon_image: entry.dpad_icon_image,
                dpad_icon_ratio: entry.dpad_icon_ratio,
                kill_icon: entry.kill_icon,
                kill_icon_image: entry.kill_icon_image,
                projectile_fx: entry.projectile_fx,
                proj_trail_from_slot: entry.proj_trail_slot.is_some(),
                proj_trail: entry.proj_trail,
                proj_beacon_from_slot: entry.proj_beacon_slot.is_some(),
                proj_beacon: entry.proj_beacon,
                proj_ignition_from_slot: entry.proj_ignition_slot.is_some(),
                proj_ignition: entry.proj_ignition,
                display_name_key: entry.display_name_key,
            });
        }
        let mut registry = WeaponRegistry {
            rows,
            world_catalog_identity: 0,
            iw5_attachments,
            iw5_candidates: Vec::new(),
            configurations: HashMap::new(),
            by_name: index_of,
            by_namespaced: HashMap::new(),
            item_groups: HashMap::new(),
            families: crate::WeaponFamilies::default(),
            fpv_clip_tracks: Arc::default(),
            revision: mint_weapon_revision(),
        };
        registry.rebuild_name_maps();
        Self {
            registry,
            combat_slots,
            family_tables: Vec::new(),
        }
    }
}

impl WeaponRegistry {
    pub fn world_catalog_identity(&self) -> u64 {
        self.world_catalog_identity
    }

    fn build_iw5_candidates(&mut self) {
        self.iw5_candidates = self
            .families
            .iw5_candidate_selections()
            .into_iter()
            .map(|(base_id, selection)| {
                let native = self.resolve_iw5_attachment_slots(base_id, &selection.attachments);
                let primary_assets = native
                    .as_ref()
                    .ok()
                    .and_then(|native| self.iw5_primary_attachment_assets(base_id, *native))
                    .map(|assets| assets.into_iter().filter_map(|a| a.scope.clone()).collect())
                    .unwrap_or_default();
                let primary_ads_zoom_fov = native
                    .as_ref()
                    .ok()
                    .and_then(|native| self.iw5_primary_ads_zoom_fov(base_id, *native));
                let primary_ads_aim_pitch = native
                    .as_ref()
                    .ok()
                    .and_then(|native| self.iw5_primary_ads_aim_pitch(base_id, *native));
                Iw5ConfigurationCandidate {
                    selection,
                    base_id,
                    native,
                    primary_assets,
                    primary_ads_zoom_fov,
                    primary_ads_aim_pitch,
                }
            })
            .collect();
    }

    pub fn iw5_configuration_candidates(&self) -> &[Iw5ConfigurationCandidate] {
        &self.iw5_candidates
    }

    pub fn iw5_attachment_slots_of(
        &self,
        id: u32,
    ) -> Option<&[Option<String>; fastfile_iw5::size::WEAPON_ATTACHMENT_SLOT_COUNT]> {
        let row = self.rows.get(id as usize)?;
        (row.namespace == crate::AssetNamespace::Iw5).then_some(&row.iw5_attachment_slots)
    }

    pub fn iw5_attachment_asset(&self, native_name: &str) -> Option<&Iw5ScopeRow> {
        self.iw5_attachments.get(native_name)
    }

    pub fn iw5_reload_overrides_of(&self, id: u32) -> Option<&[fastfile_iw5::ReloadOverride]> {
        let row = self.rows.get(id as usize)?;
        (row.namespace == crate::AssetNamespace::Iw5).then_some(row.iw5_reload_overrides.as_slice())
    }

    pub fn iw5_anim_overrides_of(&self, id: u32) -> Option<&[LeftoverAnimOverride]> {
        let row = self.rows.get(id as usize)?;
        (row.namespace == crate::AssetNamespace::Iw5).then_some(row.iw5_anim_overrides.as_slice())
    }

    pub fn iw5_fx_overrides_of(&self, id: u32) -> Option<&[Iw5FxOverride]> {
        let row = self.rows.get(id as usize)?;
        (row.namespace == crate::AssetNamespace::Iw5).then_some(row.iw5_fx_overrides.as_slice())
    }

    pub fn iw5_notetrack_overrides_of(&self, id: u32) -> Option<&[Iw5NotetrackOverride]> {
        let row = self.rows.get(id as usize)?;
        (row.namespace == crate::AssetNamespace::Iw5)
            .then_some(row.iw5_notetrack_overrides.as_slice())
    }

    pub fn select_iw5_anim_override(
        &self,
        id: u32,
        selection: Iw5AttachmentSelection,
        anim_tree_type: u32,
    ) -> Option<&LeftoverAnimOverride> {
        iw5_best_pair_override(
            self.iw5_anim_overrides_of(id)?,
            selection,
            anim_tree_type,
            |row| (row.attachment1, row.attachment2, row.anim_tree_type),
        )
    }

    pub fn select_iw5_sound_override(
        &self,
        id: u32,
        selection: Iw5AttachmentSelection,
        sound_type: u32,
    ) -> Option<&LeftoverSoundOverride> {
        let row = self.rows.get(id as usize)?;
        if row.namespace != crate::AssetNamespace::Iw5 {
            return None;
        }
        iw5_best_pair_override(
            &row.sounds.leftover_sound_overrides,
            selection,
            sound_type,
            |row| (row.attachment1, row.attachment2, row.sound_type),
        )
    }

    pub fn select_iw5_fx_override(
        &self,
        id: u32,
        selection: Iw5AttachmentSelection,
        fx_type: u32,
    ) -> Option<&Iw5FxOverride> {
        iw5_best_pair_override(self.iw5_fx_overrides_of(id)?, selection, fx_type, |row| {
            (row.attachment1, row.attachment2, row.fx_type)
        })
    }

    pub fn select_iw5_reload_override(
        &self,
        id: u32,
        selection: Iw5AttachmentSelection,
    ) -> Option<&fastfile_iw5::ReloadOverride> {
        self.iw5_reload_overrides_of(id)?
            .iter()
            .find(|row| row.attachment != 0 && selection.contains_condition(row.attachment))
    }

    pub fn select_iw5_notetrack_override(
        &self,
        id: u32,
        selection: Iw5AttachmentSelection,
    ) -> Option<&Iw5NotetrackOverride> {
        self.iw5_notetrack_overrides_of(id)?
            .iter()
            .find(|row| row.attachment != 0 && selection.contains_condition(row.attachment))
    }

    pub fn resolve_iw5_attachment_slots(
        &self,
        base_id: u32,
        names: &[String],
    ) -> Result<Iw5AttachmentSelection, crate::ConfigurationRefusal> {
        let slots = self.iw5_attachment_slots_of(base_id).ok_or_else(|| {
            crate::ConfigurationRefusal::UnknownFamily(self.name_of(base_id).to_owned())
        })?;
        let mut selected = Iw5AttachmentSelection::default();
        for name in names {
            let mut matches = slots.iter().enumerate().filter(|(_, slot)| {
                slot.as_deref()
                    .is_some_and(|native| native.eq_ignore_ascii_case(name))
            });
            let direct = matches.next();
            if matches.next().is_some() {
                return Err(crate::ConfigurationRefusal::Unsupported(format!(
                    "`{name}` has more than one native IW5 slot in `{}`",
                    self.name_of(base_id)
                )));
            }
            let index = if let Some((index, _)) = direct {
                index
            } else {
                let display_key = format!("WEAPON_{}_ATTACHMENT", name.to_ascii_uppercase());
                let mut by_display = slots.iter().enumerate().filter(|(_, slot)| {
                    slot.as_deref()
                        .and_then(|native| self.iw5_attachments.get(native))
                        .and_then(|asset| asset.display_name.as_deref())
                        .is_some_and(|key| key.eq_ignore_ascii_case(&display_key))
                });
                let Some((index, _)) = by_display.next() else {
                    return Err(crate::ConfigurationRefusal::NotOffered(name.clone()));
                };
                if by_display.next().is_some() {
                    return Err(crate::ConfigurationRefusal::Unsupported(format!(
                        "`{name}` has an ambiguous IW5 display key in `{}`",
                        self.name_of(base_id)
                    )));
                }
                index
            };
            let native = slots[index].as_deref().expect("matched native slot");
            if !self.iw5_attachments.contains_key(native) {
                return Err(crate::ConfigurationRefusal::MissingContent(
                    native.to_owned(),
                ));
            }
            match index {
                0..=5 => {
                    if selected.scope != 0 && selected.scope != (index + 1) as u8 {
                        return Err(crate::ConfigurationRefusal::Incompatible {
                            a: slots[selected.scope as usize - 1]
                                .clone()
                                .unwrap_or_default(),
                            b: name.clone(),
                        });
                    }
                    selected.scope = (index + 1) as u8;
                }
                6..=8 => {
                    if selected.underbarrel != 0 && selected.underbarrel != (index - 5) as u8 {
                        return Err(crate::ConfigurationRefusal::Incompatible {
                            a: slots[selected.underbarrel as usize + 5]
                                .clone()
                                .unwrap_or_default(),
                            b: name.clone(),
                        });
                    }
                    selected.underbarrel = (index - 5) as u8;
                }
                9..=12 => selected.others |= 1 << (index - 9),
                _ => unreachable!(),
            }
        }
        Ok(selected)
    }

    pub fn iw5_primary_attachment_assets(
        &self,
        base_id: u32,
        selection: Iw5AttachmentSelection,
    ) -> Option<Vec<&Iw5ScopeRow>> {
        let slots = self.iw5_attachment_slots_of(base_id)?;
        let slot_asset = |index: usize| {
            slots
                .get(index)?
                .as_deref()
                .and_then(|name| self.iw5_attachment_asset(name))
        };
        let mut assets = Vec::with_capacity(3);
        let mut push = |asset| {
            if assets.len() < 3 {
                assets.push(asset);
            } else {
                assets[2] = asset;
            }
        };
        if selection.scope != 0 {
            push(slot_asset(usize::from(selection.scope - 1))?);
        }
        for bit in 0..4 {
            if selection.others & (1 << bit) != 0 {
                push(slot_asset(9 + bit)?);
            }
        }
        if selection.underbarrel != 0 {
            let asset = slot_asset(usize::from(selection.underbarrel) + 5)?;
            if asset.weapon_class == 0
                || asset.ads_settings_main.is_some()
                || (asset.scales.ads_settings_main != 0.0 && asset.scales.ads_settings_main != 1.0)
            {
                push(asset);
            }
        }
        Some(assets)
    }

    pub fn iw5_primary_ads_aim_pitch(
        &self,
        base_id: u32,
        selection: Iw5AttachmentSelection,
    ) -> Option<f32> {
        self.iw5_primary_ads_value(
            base_id,
            selection,
            |facts| facts.ads_aim_pitch,
            |settings| settings.ads_aim_pitch,
        )
    }

    pub fn iw5_primary_ads_zoom_fov(
        &self,
        base_id: u32,
        selection: Iw5AttachmentSelection,
    ) -> Option<f32> {
        self.iw5_primary_ads_value(
            base_id,
            selection,
            |facts| facts.ads_zoom_fov,
            |settings| settings.ads_zoom_fov,
        )
    }

    fn iw5_primary_ads_value(
        &self,
        base_id: u32,
        selection: Iw5AttachmentSelection,
        base_value: impl FnOnce(WeaponBodyFacts) -> f32,
        setting_value: impl Fn(fastfile_iw5::AttachmentAdsSettings) -> f32,
    ) -> Option<f32> {
        let base = base_value(self.facts_of(base_id)?);
        let assets = self.iw5_primary_attachment_assets(base_id, selection)?;
        let (settings, scale) = iw5_primary_ads(&assets);
        Some(settings.map_or(base, setting_value) * scale)
    }

    fn compose_iw5_configuration(
        &self,
        base_id: u32,
        native: Iw5AttachmentSelection,
    ) -> Option<WeaponRow> {
        use fastfile_iw5::size as sz;
        let base = self.rows.get(base_id as usize)?;
        let slot_asset = |index: usize| {
            base.iw5_attachment_slots
                .get(index)?
                .as_deref()
                .and_then(|native| self.iw5_attachments.get(native))
        };
        let scope = match native.scope {
            0 => None,
            index => Some(slot_asset(usize::from(index) - 1)?),
        };
        let underbarrel = match native.underbarrel {
            0 => None,
            index => Some(slot_asset(usize::from(index) + 5)?),
        };
        let others = (0..4)
            .filter(|bit| native.others & (1 << bit) != 0)
            .map(|bit| slot_asset(9 + bit))
            .collect::<Option<Vec<_>>>()?;

        let mut row = base.clone();
        row.iw5_configuration = Some((base_id, native));

        let first = |models: &[Option<String>]| models.first().cloned().flatten();
        let mut view = if scope.is_none() {
            base.attachment_view_models.clone()
        } else {
            Vec::new()
        };
        let mut world = if scope.is_none() {
            base.attachment_world_models.clone()
        } else {
            Vec::new()
        };
        if let Some(scope) = scope {
            view.extend(first(&scope.view_models));
            view.extend(first(&scope.reticle_models));
            world.extend(first(&scope.world_models));
        }
        for asset in underbarrel.into_iter().chain(others.iter().copied()) {
            view.extend(first(&asset.view_models));
            world.extend(first(&asset.world_models));
        }
        row.attachment_view_models = view;
        row.attachment_world_models = world;

        let assets = self.iw5_primary_attachment_assets(base_id, native)?;
        apply_iw5_parameter_blocks(&mut row.facts, &assets);

        if let Some(scope) = scope {
            if let Some(overlay) = scope
                .overlay
                .as_deref()
                .filter(|name| !name.is_empty() && overlay_name_is_hud_iris(name))
            {
                row.overlay_material = Some(overlay.to_owned());
                row.overlay_image = None;
                row.overlay_material_from_slot = true;
                row.facts.ads_overlay_width = scope.width;
                row.facts.ads_overlay_height = scope.height;
            }
        }

        let mut anim_types: Vec<u32> = base
            .iw5_anim_overrides
            .iter()
            .map(|row| row.anim_tree_type)
            .collect();
        anim_types.sort_unstable();
        anim_types.dedup();
        for anim_type in anim_types {
            let Some(chosen) = self.select_iw5_anim_override(base_id, native, anim_type) else {
                continue;
            };
            let Some(slot) = iw5_anim_tree_type_to_iw4_slot(anim_type) else {
                continue;
            };
            if let Some(anim) = chosen.override_anim.clone() {
                row.sz_xanims[slot] = Some(anim);
            }
            if chosen.anim_time_ms > 0 {
                if let Some(timer) = iw5_anim_timer(&mut row.facts, slot) {
                    *timer = chosen.anim_time_ms;
                }
            }
        }

        for (sound_type, target) in [
            (sz::SND_OVERRIDE_TYPE_FIRE, &mut row.sounds.fire),
            (
                sz::SND_OVERRIDE_TYPE_PLAYER_FIRE,
                &mut row.sounds.fire_player,
            ),
            (
                sz::SND_OVERRIDE_TYPE_PLAYER_AKIMBO,
                &mut row.sounds.fire_player_akimbo,
            ),
            (
                sz::SND_OVERRIDE_TYPE_PLAYER_LASTSHOT,
                &mut row.sounds.fire_last_player,
            ),
        ] {
            if let Some(sound) = self
                .select_iw5_sound_override(base_id, native, sound_type)
                .and_then(|chosen| chosen.override_sound.clone())
            {
                *target = Some(sound);
            }
        }

        for (fx_type, hint) in [
            (1, &mut row.combat_fx.view_flash_hint),
            (2, &mut row.combat_fx.world_flash_hint),
            (3, &mut row.combat_fx.view_shell_eject_hint),
            (4, &mut row.combat_fx.world_shell_eject_hint),
        ] {
            if let Some(fx) = self
                .select_iw5_fx_override(base_id, native, fx_type)
                .and_then(|chosen| chosen.override_fx.clone())
            {
                *hint = Some(fx);
            }
        }

        if let Some(reload) = self.select_iw5_reload_override(base_id, native) {
            row.facts.reload_add_time_ms = reload.reload_add_time_ms;
            row.facts.reload_start_add_time_ms = reload.reload_start_add_time_ms;
        }
        if let Some(notetracks) = self.select_iw5_notetrack_override(base_id, native) {
            row.sounds.notetrack_sound_map = notetracks.sound_map.clone();
        }
        Some(row)
    }

    pub fn iw5_configuration_of(&self, id: u32) -> Option<(u32, Iw5AttachmentSelection)> {
        self.rows.get(id as usize)?.iw5_configuration
    }

    fn rebuild_name_maps(&mut self) {
        self.by_name.clear();
        self.by_namespaced.clear();
        for (id, row) in self.rows.iter().enumerate().skip(1) {
            if row.iw5_configuration.is_some() {
                continue;
            }
            self.by_namespaced
                .insert((row.namespace, row.name.clone()), id as u32);
            self.by_name.entry(row.name.clone()).or_insert(id as u32);
        }
    }

    pub fn reticle_of(&self, index: u32) -> Option<&WeaponReticleAssets> {
        self.rows.get(index as usize).map(|row| &row.reticle)
    }

    pub fn projectile_fx_of(&self, index: u32) -> Option<WeaponProjectileFx> {
        self.rows.get(index as usize).map(|row| row.projectile_fx)
    }

    pub fn projectile_fx_edge_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for row in self.rows.iter().skip(1) {
            for edge in row.projectile_fx.edges() {
                census.push(edge);
            }
        }
        census
    }

    pub fn sz_xanim_edges_of(
        &self,
        index: u32,
    ) -> Option<&[AssetEdge<XAnimSpace>; WEAPON_ANIM_SLOTS]> {
        self.rows.get(index as usize).map(|row| &row.sz_xanim_edges)
    }

    pub fn notetrack_action_of(&self, index: u32, note: &str) -> Option<&LinkedNotetrackAction> {
        self.rows
            .get(index as usize)?
            .notetrack_actions
            .get(&note.to_ascii_lowercase())
    }

    pub fn notetrack_sound_aliases_of(&self, index: u32) -> impl Iterator<Item = &str> {
        self.rows
            .get(index as usize)
            .into_iter()
            .flat_map(|row| row.notetrack_actions.values())
            .filter_map(|action| action.sound_alias.as_deref())
    }

    pub fn sz_xanim_right_edges_of(
        &self,
        index: u32,
    ) -> Option<&[AssetEdge<XAnimSpace>; WEAPON_ANIM_SLOTS]> {
        self.rows
            .get(index as usize)
            .map(|row| &row.sz_xanim_right_edges)
    }

    pub fn sz_xanim_left_edges_of(
        &self,
        index: u32,
    ) -> Option<&[AssetEdge<XAnimSpace>; WEAPON_ANIM_SLOTS]> {
        self.rows
            .get(index as usize)
            .map(|row| &row.sz_xanim_left_edges)
    }

    pub fn sz_xanim_edge_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for row in self.rows.iter().skip(1) {
            for edge in &row.sz_xanim_edges {
                census.push(*edge);
            }
        }
        census
    }

    pub fn bound_weapon_xanim_indices(&self) -> Vec<usize> {
        let mut indices = std::collections::BTreeSet::new();
        for row in self.rows.iter().skip(1) {
            for edge in row
                .sz_xanim_edges
                .iter()
                .chain(&row.sz_xanim_right_edges)
                .chain(&row.sz_xanim_left_edges)
            {
                if let Some(index) = edge.bound_index() {
                    indices.insert(index);
                }
            }
        }
        indices.into_iter().collect()
    }

    pub fn gun_xmodel_edge_of(&self, index: u32) -> Option<AssetEdge<FpvMeshSpace>> {
        self.rows.get(index as usize).map(|row| row.gun_xmodel_edge)
    }

    pub fn hand_xmodel_edge_of(&self, index: u32) -> Option<AssetEdge<FpvMeshSpace>> {
        self.rows
            .get(index as usize)
            .map(|row| row.hand_xmodel_edge)
    }

    pub fn rocket_model_edge_of(&self, index: u32) -> Option<AssetEdge<FpvMeshSpace>> {
        self.rows
            .get(index as usize)
            .map(|row| row.rocket_model_edge)
    }

    pub fn attachment_view_model_edges_of(&self, index: u32) -> &[AssetEdge<FpvMeshSpace>] {
        self.rows
            .get(index as usize)
            .map_or(&[], |row| row.attachment_view_model_edges.as_slice())
    }

    pub fn fpv_hands_of(
        &self,
        index: u32,
        axis: bool,
    ) -> Option<(&asset_model::FpvHands, crate::FpvMeshIndex)> {
        self.rows.get(index as usize)?.fpv_hands[usize::from(axis)]
            .as_ref()
            .map(|(choice, index)| (choice, *index))
    }

    pub fn fpv_mount_plan_of(&self, index: u32) -> Option<&asset_model::FpvMountPlan> {
        self.rows
            .get(index as usize)?
            .fpv_mount_plan
            .as_ref()?
            .as_ref()
            .ok()
    }

    pub fn fpv_assemblies_of(&self, index: u32, axis: bool) -> Option<&crate::FpvSideAssemblies> {
        self.rows.get(index as usize)?.fpv_assemblies[usize::from(axis)]
            .as_ref()?
            .as_ref()
            .ok()
    }

    pub fn fpv_assembly_gap_of(&self, index: u32, axis: bool) -> String {
        let Some(row) = self.rows.get(index as usize) else {
            return "no such weapon".to_owned();
        };
        match (&row.fpv_mount_plan, &row.fpv_assemblies[usize::from(axis)]) {
            (None, _) => "unlinked selected models".to_owned(),
            (Some(Err(error)), _) => error.to_string(),
            (_, None) => "effective kit / weapon hands".to_owned(),
            (_, Some(Err(error))) => error.clone(),
            (_, Some(Ok(_))) => "linked".to_owned(),
        }
    }

    pub fn fpv_clip_tracks(&self) -> &crate::FpvClipTracks {
        &self.fpv_clip_tracks
    }

    pub fn fpv_mount_error_of(&self, index: u32) -> Option<&asset_model::FpvMountError> {
        self.rows
            .get(index as usize)?
            .fpv_mount_plan
            .as_ref()?
            .as_ref()
            .err()
    }

    pub fn gun_xmodel_edge_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for row in self.rows.iter().skip(1) {
            census.push(row.gun_xmodel_edge);
        }
        census
    }

    pub fn world_model_edge_of(&self, index: u32) -> Option<AssetEdge<WorldWeaponSpace>> {
        self.rows
            .get(index as usize)
            .map(|row| row.world_model_edge)
    }

    pub fn attachment_world_model_edges_of(&self, index: u32) -> &[AssetEdge<WorldWeaponSpace>] {
        self.rows
            .get(index as usize)
            .map_or(&[], |row| row.attachment_world_model_edges.as_slice())
    }

    pub fn attachment_world_mounts_of(&self, index: u32) -> &[Option<String>] {
        self.rows
            .get(index as usize)
            .map_or(&[], |row| row.attachment_world_mounts.as_slice())
    }

    pub fn world_model_edge_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for row in self.rows.iter().skip(1) {
            census.push(row.world_model_edge);
        }
        census
    }

    pub fn dependency_gaps(&self) -> Vec<WeaponDependencyGap> {
        (1..self.rows.len() as u32)
            .flat_map(|id| self.dependency_gaps_of(id))
            .collect()
    }

    pub fn dependency_gaps_of(&self, id: u32) -> Vec<WeaponDependencyGap> {
        let Some(row) = self.rows.get(id as usize).filter(|_| id != 0) else {
            return Vec::new();
        };
        let named = [
            (
                row.gun_xmodel.is_some() && !row.gun_xmodel_edge.is_bound(),
                "view model",
                &row.gun_xmodel,
            ),
            (
                row.hand_xmodel.is_some() && !row.hand_xmodel_edge.is_bound(),
                "hands",
                &row.hand_xmodel,
            ),
            (
                row.rocket_model.is_some() && !row.rocket_model_edge.is_bound(),
                "FPV rocket model",
                &row.rocket_model,
            ),
            (
                row.world_model_edge.is_unresolved(),
                "world model",
                &row.world_model,
            ),
        ];
        let anims = row
            .sz_xanim_edges
            .iter()
            .zip(&row.sz_xanims)
            .map(|(edge, name)| (edge.is_unresolved(), "anim", name));
        let dual_anims = !row.facts.no_dual_wield
            && row.sz_xanims_right[weap_anim::IDLE]
                .as_deref()
                .is_some_and(|name| !name.is_empty());
        let right_anims = row
            .sz_xanim_right_edges
            .iter()
            .zip(&row.sz_xanims_right)
            .map(|(edge, name)| (dual_anims && edge.is_unresolved(), "right anim", name));
        let left_anims = row
            .sz_xanim_left_edges
            .iter()
            .zip(&row.sz_xanims_left)
            .map(|(edge, name)| (dual_anims && edge.is_unresolved(), "left anim", name));
        let mut gaps: Vec<WeaponDependencyGap> = named
            .into_iter()
            .chain(anims)
            .chain(right_anims)
            .chain(left_anims)
            .filter(|(unresolved, ..)| *unresolved)
            .map(|(_, kind, name)| WeaponDependencyGap {
                id,
                kind,
                name: name.clone().unwrap_or_default(),
            })
            .collect();
        if let Some(Err(error)) = &row.fpv_mount_plan {
            gaps.push(WeaponDependencyGap {
                id,
                kind: "FPV mount",
                name: error.to_string(),
            });
        }
        for (side, assembly) in row.fpv_assemblies.iter().enumerate() {
            if let Some(Err(error)) = assembly {
                gaps.push(WeaponDependencyGap {
                    id,
                    kind: "FPV skeleton",
                    name: format!("{}: {error}", if side == 0 { "allies" } else { "axis" }),
                });
            }
        }
        if row.gun_xmodel_edge.is_bound() {
            for (side, hands) in row.fpv_hands.iter().enumerate() {
                if hands.is_none() {
                    gaps.push(WeaponDependencyGap {
                        id,
                        kind: "FPV hands layout",
                        name: if side == 0 { "allies" } else { "axis" }.to_owned(),
                    });
                }
            }
        }
        for (name, edge) in row
            .attachment_view_models
            .iter()
            .zip(&row.attachment_view_model_edges)
        {
            if !edge.is_bound() {
                gaps.push(WeaponDependencyGap {
                    id,
                    kind: "FPV attachment model",
                    name: name.clone(),
                });
            }
        }
        for (name, edge) in row
            .attachment_world_models
            .iter()
            .zip(&row.attachment_world_model_edges)
        {
            if !edge.is_bound() {
                gaps.push(WeaponDependencyGap {
                    id,
                    kind: "world attachment model",
                    name: name.clone(),
                });
            }
        }
        for ((name, edge), mount) in row
            .attachment_world_models
            .iter()
            .zip(&row.attachment_world_model_edges)
            .zip(&row.attachment_world_mounts)
        {
            if edge.is_bound() && mount.is_none() {
                gaps.push(WeaponDependencyGap {
                    id,
                    kind: "world attachment mount",
                    name: name.clone(),
                });
            }
        }
        if row.attachment_world_models.len() != row.attachment_world_model_edges.len() {
            gaps.push(WeaponDependencyGap {
                id,
                kind: "world attachment bindings",
                name: format!(
                    "{} names, {} links",
                    row.attachment_world_models.len(),
                    row.attachment_world_model_edges.len()
                ),
            });
        }
        if row.attachment_view_models.len() != row.attachment_view_model_edges.len() {
            gaps.push(WeaponDependencyGap {
                id,
                kind: "FPV attachment bindings",
                name: format!(
                    "{} names, {} links",
                    row.attachment_view_models.len(),
                    row.attachment_view_model_edges.len()
                ),
            });
        }
        gaps
    }

    pub fn configuration_admission(&self, id: u32) -> Result<(), crate::ConfigurationRefusal> {
        if !self.configuration_supported(id) {
            return Err(crate::ConfigurationRefusal::Unsupported(format!(
                "`{}` needs a runtime mechanism IW4L lacks",
                self.name_of(id)
            )));
        }
        match self.dependency_gaps_of(id).first() {
            Some(gap) => Err(crate::ConfigurationRefusal::MissingDependency {
                weapon: self.name_of(id).to_owned(),
                kind: gap.kind,
                name: gap.name.clone(),
            }),
            None => Ok(()),
        }
    }

    pub fn world_model_entry<'a>(
        &self,
        index: u32,
        catalog: &'a crate::WorldWeaponCatalog,
    ) -> Option<&'a crate::WorldWeaponEntry> {
        if self.world_catalog_identity != catalog.identity() {
            return None;
        }
        catalog.get_at(self.world_model_edge_of(index)?.bound_index()?)
    }

    pub fn authored_weapon_sound(&self, index: u32, slot: WeaponSoundSlot) -> Option<&str> {
        self.sounds_of(index)?
            .hint(slot)
            .filter(|name| !name.is_empty())
    }

    pub fn weapon_sound_alias<'a>(
        &self,
        index: u32,
        slot: WeaponSoundSlot,
        catalog: &'a crate::SoundCatalog,
    ) -> Option<&'a str> {
        self.weapon_sound_key(index, slot, catalog)
            .map(|(_, alias)| alias)
    }

    pub fn weapon_sound_key<'a>(
        &self,
        index: u32,
        slot: WeaponSoundSlot,
        catalog: &'a crate::SoundCatalog,
    ) -> Option<(crate::AssetNamespace, &'a str)> {
        let ns = self.namespace_of(index).unwrap_or_default();
        sound_alias_in_bank(self.authored_weapon_sound(index, slot), ns, catalog)
    }

    pub fn bounce_sound_alias<'a>(
        &self,
        index: u32,
        surf: usize,
        catalog: &'a crate::SoundCatalog,
    ) -> Option<&'a str> {
        let ns = self.namespace_of(index).unwrap_or_default();
        sound_alias_in_bank(self.bounce_sound_of(index, surf), ns, catalog).map(|(_, alias)| alias)
    }

    pub fn hud_material_edge_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for row in &self.rows {
            census.push(row.reticle.center_edge);
            census.push(row.reticle.side_edge);
        }
        for row in &self.rows {
            census.push(row.hud_material_edges.overlay);
            census.push(row.hud_material_edges.hud_icon);
            census.push(row.hud_material_edges.pickup_icon);
            census.push(row.hud_material_edges.kill_icon);
        }
        census
    }

    pub fn display_name_key_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.display_name_key.as_deref())
    }

    pub fn facts_of(&self, index: u32) -> Option<WeaponBodyFacts> {
        self.rows.get(index as usize).map(|row| row.facts)
    }

    pub fn resolve_index(&self, name: &str) -> Result<Option<u32>, UnknownWeaponName> {
        if name.is_empty() {
            return Ok(None);
        }
        if name.contains(':') {
            match crate::AssetKey::parse(name) {
                Ok(key) => return self.resolve_key(&key),
                Err(_) => {
                    return Err(UnknownWeaponName {
                        name: name.to_owned(),
                    });
                }
            }
        }
        let norm = normalize_weapon_name(name);
        match self.by_name.get(&norm).copied() {
            Some(id) => Ok(Some(id)),
            None => Err(UnknownWeaponName { name: norm }),
        }
    }

    pub fn resolve_key(&self, key: &crate::AssetKey) -> Result<Option<u32>, UnknownWeaponName> {
        if key.kind != crate::AssetKind::Weapon {
            return Err(UnknownWeaponName {
                name: key.display(),
            });
        }
        let norm = normalize_weapon_name(key.logical_name());
        match self.by_namespaced.get(&(key.namespace, norm)).copied() {
            Some(id) => Ok(Some(id)),
            None => Err(UnknownWeaponName {
                name: key.display(),
            }),
        }
    }

    pub fn namespace_of(&self, index: u32) -> Option<crate::AssetNamespace> {
        if index == 0 {
            return None;
        }
        self.rows.get(index as usize).map(|row| row.namespace)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn item_group_of(&self, index: u32) -> Option<&str> {
        let ns = self.namespace_of(index)?;
        let name = self.name_of(index);
        if name.is_empty() {
            return None;
        }
        self.item_groups
            .get(&(ns, name.to_owned()))
            .map(String::as_str)
    }

    #[must_use]
    pub fn item_group_count(&self) -> usize {
        self.item_groups.len()
    }

    pub fn namespaced_key_of(&self, index: u32) -> Option<String> {
        self.key_of(index).map(|key| key.to_string())
    }

    pub fn key_of(&self, index: u32) -> Option<crate::AssetKey> {
        let ns = self.namespace_of(index)?;
        let name = self.name_of(index);
        if name.is_empty() {
            return None;
        }
        crate::AssetKey::new(ns, crate::AssetKind::Weapon, gsc_weapon_script_name(name)).ok()
    }

    pub fn namespace_count(&self, ns: crate::AssetNamespace) -> usize {
        self.rows
            .iter()
            .skip(1)
            .filter(|row| row.namespace == ns)
            .count()
    }

    #[deprecated(note = "use resolve_index — unknown must not become 0")]
    pub fn index_of(&self, name: &str) -> u32 {
        match self.resolve_index(name) {
            Ok(Some(id)) => id,
            Ok(None) | Err(_) => 0,
        }
    }

    pub fn name_of(&self, index: u32) -> &str {
        self.rows
            .get(index as usize)
            .map(|row| row.name.as_str())
            .unwrap_or("")
    }

    pub fn script_name_of(&self, index: u32) -> String {
        gsc_weapon_script_name(self.name_of(index))
    }

    pub fn script_names_table(&self) -> Vec<String> {
        (0..self.rows.len())
            .map(|i| self.script_name_of(i as u32))
            .collect()
    }

    pub fn configuration_supported(&self, id: u32) -> bool {
        if self.namespace_of(id) != Some(crate::AssetNamespace::T5) {
            return true;
        }
        let dual_hand_model = self
            .gun_xmodel_of(id)
            .is_some_and(|name| name.ends_with("_dw_rh") || name.ends_with("_dw_lh"));
        !dual_hand_model
    }

    pub fn runnable_table(&self) -> Vec<bool> {
        (0..self.rows.len() as u32)
            .map(|id| id != 0 && self.configuration_admission(id).is_ok())
            .collect()
    }

    pub fn weapon_families(&self) -> &crate::WeaponFamilies {
        &self.families
    }

    pub fn resolve_configuration(
        &self,
        selection: &crate::WeaponSelection,
        rules: crate::LoadoutRules,
    ) -> Result<crate::ResolvedConfiguration, crate::ConfigurationRefusal> {
        self.families.resolve(selection, rules, self)
    }

    pub fn describe_configuration(&self, id: u32) -> Option<&crate::WeaponSelection> {
        self.families.describe(id)
    }

    pub fn prepared_attachments_of(&self, id: u32) -> &[String] {
        self.rows
            .get(id as usize)
            .map_or(&[], |row| row.prepared_attachments.as_slice())
    }

    pub fn configuration_label(&self, id: u32) -> String {
        match self
            .describe_configuration(id)
            .and_then(|selection| Some((selection.family.as_ref()?, &selection.attachments)))
        {
            Some((family, attachments)) => {
                let mut label = family.short();
                for name in attachments {
                    label.push_str(" +");
                    label.push_str(name);
                }
                label
            }
            None => self.namespaced_key_of(id).unwrap_or_default(),
        }
    }

    pub fn configuration_transition_groups(&self) -> Vec<u32> {
        let mut keys: Vec<_> = self
            .families
            .families()
            .iter()
            .map(|family| family.key.clone())
            .collect();
        keys.sort_by_key(|key| key.asset_key());
        keys.dedup();
        let groups: HashMap<_, _> = keys
            .into_iter()
            .enumerate()
            .map(|(index, key)| (key, index as u32 + 1))
            .collect();
        let mut result = vec![0; self.rows.len()];
        for id in 1..self.rows.len() as u32 {
            let Some(selection) = self.describe_configuration(id) else {
                continue;
            };
            let Some(key) = selection.family.as_ref() else {
                continue;
            };
            if self
                .resolve_configuration(selection, crate::LoadoutRules::default())
                .is_ok_and(|resolved| resolved.id == id)
            {
                result[id as usize] = groups.get(key).copied().unwrap_or(0);
            }
        }
        result
    }

    pub fn list_attachment_choices(
        &self,
        selection: &crate::WeaponSelection,
        rules: crate::LoadoutRules,
    ) -> Result<Vec<crate::AttachmentOption>, crate::ConfigurationRefusal> {
        self.families.attachment_options(selection, rules, self)
    }

    pub fn gun_xmodel_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.gun_xmodel.as_deref())
    }

    pub fn hand_xmodel_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.hand_xmodel.as_deref())
    }

    pub fn world_model_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.world_model.as_deref())
    }

    pub fn projectile_model_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.projectile_model.as_deref())
    }

    pub fn rocket_model_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.rocket_model.as_deref())
    }

    pub fn anim_of(&self, index: u32, slot: usize) -> Option<&str> {
        self.rows
            .get(index as usize)?
            .sz_xanims
            .get(slot)?
            .as_deref()
    }

    pub fn idle_anim_of(&self, index: u32) -> Option<&str> {
        self.anim_of(index, weap_anim::IDLE)
    }

    pub fn hide_tags_of(&self, index: u32) -> &[String] {
        self.rows
            .get(index as usize)
            .map(|row| row.hide_tags.as_slice())
            .unwrap_or(&[])
    }

    pub fn attachment_view_models_of(&self, index: u32) -> &[String] {
        self.rows
            .get(index as usize)
            .map_or(&[], |row| row.attachment_view_models.as_slice())
    }

    pub fn attachment_world_models_of(&self, index: u32) -> &[String] {
        self.rows
            .get(index as usize)
            .map_or(&[], |row| row.attachment_world_models.as_slice())
    }

    pub fn sounds_of(&self, index: u32) -> Option<&WeaponSoundAliases> {
        self.rows.get(index as usize).map(|row| &row.sounds)
    }

    pub fn bounce_sound_of(&self, index: u32, surf: usize) -> Option<&str> {
        self.sounds_of(index)?.bounce.get(surf)?.as_deref()
    }

    pub fn combat_fx_of(&self, index: u32) -> Option<&WeaponCombatFx> {
        self.rows.get(index as usize).map(|row| &row.combat_fx)
    }

    pub fn tracer_type_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for row in self.rows.iter().skip(1) {
            census.push(row.combat_fx.tracer);
        }
        census
    }

    pub fn combat_fx_census(&self) -> AssetEdgeCensus {
        let mut census = AssetEdgeCensus::default();
        for row in self.rows.iter().skip(1) {
            for edge in row.combat_fx.fx_edges() {
                census.push(edge);
            }
        }
        census
    }

    pub fn combat_fx_unresolved_names(&self) -> Vec<&str> {
        self.rows
            .iter()
            .skip(1)
            .filter(|row| {
                row.combat_fx
                    .fx_edges()
                    .iter()
                    .any(|edge| edge.is_unresolved())
            })
            .map(|row| row.name.as_str())
            .collect()
    }

    pub fn tracer_type_unresolved_names(&self) -> Vec<&str> {
        self.rows
            .iter()
            .skip(1)
            .filter(|row| row.combat_fx.tracer.is_unresolved())
            .map(|row| row.name.as_str())
            .collect()
    }

    pub fn weap_def_of(&self, index: u32) -> Option<(u8, u32)> {
        self.rows.get(index as usize).and_then(|row| row.weap_def)
    }

    pub fn sz_xanims_of(&self, index: u32) -> Option<&[Option<String>; WEAPON_ANIM_SLOTS]> {
        self.rows.get(index as usize).map(|row| &row.sz_xanims)
    }

    pub fn sz_xanims_right_of(&self, index: u32) -> Option<&[Option<String>; WEAPON_ANIM_SLOTS]> {
        self.rows
            .get(index as usize)
            .map(|row| &row.sz_xanims_right)
    }

    pub fn sz_xanims_left_of(&self, index: u32) -> Option<&[Option<String>; WEAPON_ANIM_SLOTS]> {
        self.rows.get(index as usize).map(|row| &row.sz_xanims_left)
    }

    pub fn idle_anim_right_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.sz_xanims_right[weap_anim::IDLE].as_deref())
            .filter(|s| !s.is_empty())
    }

    pub fn idle_anim_left_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.sz_xanims_left[weap_anim::IDLE].as_deref())
            .filter(|s| !s.is_empty())
    }

    pub fn timers_of(&self, index: u32) -> (i32, i32) {
        self.facts_of(index)
            .map(|f| (f.fire_time_ms, f.raise_time_ms))
            .unwrap_or((0, 0))
    }

    pub fn switch_timers_of(&self, index: u32) -> (i32, i32, i32) {
        self.facts_of(index)
            .map(|f| (f.drop_time_ms, f.quick_drop_time_ms, f.quick_raise_time_ms))
            .unwrap_or((0, 0, 0))
    }

    pub fn sprint_timers_of(&self, index: u32) -> (i32, i32, i32) {
        self.facts_of(index)
            .map(|f| {
                (
                    f.sprint_raise_time_ms,
                    f.sprint_loop_time_ms,
                    f.sprint_drop_time_ms,
                )
            })
            .unwrap_or((0, 0, 0))
    }

    pub fn quick_reload_timers_of(&self, index: u32) -> Option<(i32, i32)> {
        let dual_mag = self.facts_of(index)?.dual_mag?;
        Some((dual_mag.reload_ms, dual_mag.reload_empty_ms))
    }

    pub fn reload_timers_of(&self, index: u32) -> (i32, i32, i32, i32) {
        self.facts_of(index)
            .map(|f| {
                (
                    f.reload_time_ms,
                    f.reload_empty_time_ms,
                    f.reload_start_time_ms,
                    f.reload_end_time_ms,
                )
            })
            .unwrap_or((0, 0, 0, 0))
    }

    pub fn len(&self) -> usize {
        self.rows.len().saturating_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn scales_table(&self) -> Vec<(f32, f32, f32)> {
        self.rows
            .iter()
            .map(|row| {
                (
                    row.facts.move_speed_scale,
                    row.facts.ads_move_speed_scale,
                    row.facts.sprint_duration_scale,
                )
            })
            .collect()
    }

    pub fn gun_xmodel_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.gun_xmodel.is_some())
            .count()
    }

    pub fn overlay_material_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.overlay_material.as_deref())
    }

    pub fn overlay_image_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.overlay_image.as_deref())
    }

    pub fn overlay_is_hud_iris(&self, index: u32) -> bool {
        self.overlay_material_of(index)
            .is_some_and(overlay_name_is_hud_iris)
    }

    pub fn pickup_icon_of(&self, index: u32) -> Option<(&str, i32)> {
        let row = self.rows.get(index as usize)?;
        if row.pickup_icon_authored {
            Some((row.pickup_icon_image.as_deref()?, row.pickup_icon_ratio))
        } else {
            Some((row.hud_icon_image.as_deref()?, row.hud_icon_ratio))
        }
    }

    pub fn hud_icon_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.hud_icon.as_deref())
    }

    pub fn hud_icon_image_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.hud_icon_image.as_deref())
    }

    pub fn dpad_icon_of(&self, index: u32) -> Option<(&str, i32)> {
        let row = self.rows.get(index as usize)?;
        Some((row.dpad_icon_image.as_deref()?, row.dpad_icon_ratio))
    }

    pub fn kill_icon_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.kill_icon.as_deref())
    }

    pub fn kill_icon_image_of(&self, index: u32) -> Option<&str> {
        self.rows
            .get(index as usize)
            .and_then(|row| row.kill_icon_image.as_deref())
    }

    pub fn proj_trail_of(&self, index: u32) -> Option<&str> {
        let row = self.rows.get(index as usize)?;
        row.projectile_fx.trail.is_bound().then_some(())?;
        row.proj_trail.as_deref()
    }

    pub fn proj_beacon_of(&self, index: u32) -> Option<&str> {
        let row = self.rows.get(index as usize)?;
        row.projectile_fx.beacon.is_bound().then_some(())?;
        row.proj_beacon.as_deref()
    }

    pub fn proj_ignition_of(&self, index: u32) -> Option<&str> {
        let row = self.rows.get(index as usize)?;
        row.projectile_fx.ignition.is_bound().then_some(())?;
        row.proj_ignition.as_deref()
    }

    pub fn world_model_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.world_model.is_some())
            .count()
    }

    pub fn projectile_model_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.projectile_model.is_some())
            .count()
    }

    pub fn projectile_model_edge_of(&self, index: u32) -> Option<AssetEdge<ProjectileModelSpace>> {
        self.rows
            .get(index as usize)
            .map(|row| row.projectile_model_edge)
    }

    pub fn projectile_model_bound_n(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.projectile_model_edge.is_bound())
            .count()
    }

    pub fn projectile_model_name_hint_n(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.projectile_model.is_some() && !row.projectile_model_edge.is_bound())
            .count()
    }

    pub fn projectile_model_unresolved_hints(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|row| !row.projectile_model_edge.is_bound())
            .filter_map(|row| row.projectile_model.as_deref())
            .collect()
    }

    pub fn idle_anim_count(&self) -> usize {
        (1..=self.len() as u32)
            .filter(|&id| self.idle_anim_of(id).is_some())
            .count()
    }

    pub fn sz_xanims_count(&self) -> usize {
        self.rows
            .iter()
            .skip(1)
            .filter(|row| row.sz_xanims.iter().any(|n| n.is_some()))
            .count()
    }
}

fn iw5_primary_ads(assets: &[&Iw5ScopeRow]) -> (Option<fastfile_iw5::AttachmentAdsSettings>, f32) {
    let mut chosen = None;
    let mut scale_product = 1.0;
    for asset in assets {
        let settings = if asset.share_ammo_with_alt {
            asset.ads_settings_main
        } else {
            asset.ads_settings
        };
        if chosen.is_none() {
            chosen = settings;
        }
        let scale = if asset.share_ammo_with_alt {
            (asset.ads_settings_main.is_none()
                && asset.scales.ads_settings_main != 0.0
                && asset.scales.ads_settings_main != 1.0)
                .then_some(asset.scales.ads_settings_main)
        } else {
            (asset.scales.ads_settings > 0.0).then_some(asset.scales.ads_settings)
        };
        if let Some(scale) = scale {
            scale_product *= scale;
        }
    }
    (chosen, scale_product)
}

fn iw5_default_scope_models(
    slots: &[Option<String>; fastfile_iw5::size::WEAPON_ATTACHMENT_SLOT_COUNT],
    attachments: &HashMap<String, Iw5ScopeRow>,
) -> Option<(Option<String>, Option<String>)> {
    slots[..6]
        .iter()
        .filter_map(Option::as_deref)
        .find_map(|name| {
            if !name.ends_with("scope") || name.ends_with("vzscope") {
                return None;
            }
            let asset = attachments.get(name)?;
            let view = asset.view_models[0].clone();
            let world = asset.world_models[0].clone();
            (view.is_some() || world.is_some()).then_some((view, world))
        })
}

fn iw5_first_block<T: Copy>(
    assets: &[&Iw5ScopeRow],
    block: impl Fn(&Iw5ScopeRow) -> Option<T>,
) -> Option<T> {
    assets.iter().find_map(|asset| block(asset))
}

fn iw5_scale_product(
    assets: &[&Iw5ScopeRow],
    scale: impl Fn(&fastfile_iw5::AttachmentScales) -> f32,
) -> f32 {
    assets
        .iter()
        .map(|asset| scale(&asset.scales))
        .filter(|scale| *scale > 0.0)
        .product()
}

fn scale_i32(value: i32, scale: f32) -> i32 {
    if scale == 1.0 {
        value
    } else {
        (value as f32 * scale) as i32
    }
}

fn apply_iw5_parameter_blocks(facts: &mut WeaponBodyFacts, assets: &[&Iw5ScopeRow]) {
    let (ads, ads_scale) = iw5_primary_ads(assets);
    if let Some(ads) = ads {
        facts.ads_spread = ads.ads_spread;
        facts.ads_aim_pitch = ads.ads_aim_pitch;
        facts.ads_crosshair_in_frac = ads.ads_crosshair_in_frac;
        facts.ads_crosshair_out_frac = ads.ads_crosshair_out_frac;
        facts.ads_zoom_fov = ads.ads_zoom_fov;
        facts.ads_zoom_in_frac = ads.ads_zoom_in_frac;
        facts.ads_zoom_out_frac = ads.ads_zoom_out_frac;
        facts.ads_bob_factor_at_0x330 = ads.ads_bob_factor;
        facts.ads_view_bob_mult_at_0x334 = ads.ads_view_bob_mult;
        if ads.ads_trans_in_time > 0.0 {
            facts.ads_in_rate = 1.0 / ads.ads_trans_in_time;
        }
        if ads.ads_trans_out_time > 0.0 {
            facts.ads_out_rate = 1.0 / ads.ads_trans_out_time;
        }
    }
    if ads_scale != 1.0 {
        facts.ads_spread *= ads_scale;
        facts.ads_aim_pitch *= ads_scale;
        facts.ads_zoom_fov *= ads_scale;
        facts.ads_in_rate /= ads_scale;
        facts.ads_out_rate /= ads_scale;
    }

    if let Some(sight) = iw5_first_block(assets, |a| a.sight) {
        facts.aim_down_sight = sight.aim_down_sight;
        facts.ads_fire_only = sight.ads_fire;
        facts.rechamber_while_ads = sight.rechamber_while_ads;
        facts.no_ads_when_mag_empty = sight.no_ads_when_mag_empty;
    }
    if let Some(general) = iw5_first_block(assets, |a| a.ammo_general) {
        facts.penetrate_type = general.penetrate_type;
        facts.penetrate_multiplier = general.penetrate_multiplier;
        facts.impact_type = general.impact_type;
        facts.fire_type = general.fire_type;
        facts.rifle_bullet = general.rifle_bullet;
    }
    if let Some(reload) = iw5_first_block(assets, |a| a.reload) {
        facts.no_partial_reload = reload.no_partial_reload;
        facts.segmented_reload = reload.segmented_reload;
    }
    if let Some(add_ons) = iw5_first_block(assets, |a| a.add_ons) {
        facts.motion_tracker = add_ons.motion_tracker;
    }
    if let Some(general) = iw5_first_block(assets, |a| a.general) {
        facts.bolt_action = general.bolt_action;
        facts.inherits_perks = general.inherits_perks;
        facts.move_speed_scale = general.move_speed_scale;
        facts.ads_move_speed_scale = general.ads_move_speed_scale;
    }

    if let Some(ammo) = iw5_first_block(assets, |a| a.ammunition) {
        facts.max_ammo = ammo.max_ammo;
        facts.start_ammo = ammo.start_ammo;
        facts.clip_size = ammo.clip_size;
        facts.shots_per_fire = ammo.shot_count;
        facts.reload_ammo_add = ammo.reload_ammo_add;
        facts.reload_start_add = ammo.reload_start_add;
    }
    let ammo_scale = iw5_scale_product(assets, |s| s.ammunition);
    facts.max_ammo = scale_i32(facts.max_ammo, ammo_scale);
    facts.start_ammo = scale_i32(facts.start_ammo, ammo_scale);
    facts.clip_size = scale_i32(facts.clip_size, ammo_scale);

    if let Some(damage) = iw5_first_block(assets, |a| a.damage) {
        facts.damage = damage.damage;
        facts.min_damage = damage.min_damage;
        facts.melee_damage = damage.melee_damage;
        facts.max_damage_range = damage.max_damage_range;
        facts.min_damage_range = damage.min_damage_range;
        facts.min_player_damage = damage.min_player_damage;
    }
    facts.damage = scale_i32(facts.damage, iw5_scale_product(assets, |s| s.damage));
    let damage_min = iw5_scale_product(assets, |s| s.damage_min);
    facts.min_damage = scale_i32(facts.min_damage, damage_min);
    facts.min_player_damage = scale_i32(facts.min_player_damage, damage_min);

    if let Some(location) = iw5_first_block(assets, |a| a.location_damage) {
        let mut mult = facts.location_damage_mult.unwrap_or([1.0; 20]);
        mult[..19].copy_from_slice(&location);
        facts.location_damage_mult = Some(mult);
    }

    if let Some(idle) = iw5_first_block(assets, |a| a.idle_settings) {
        facts.idle.hip_idle_amount_at_0x370 = idle.hip_idle_amount;
        facts.idle.hip_idle_speed_at_0x378 = idle.hip_idle_speed;
        facts.idle.idle_crouch_factor_at_0x37c = idle.idle_crouch_factor;
        facts.idle.idle_prone_factor_at_0x380 = idle.idle_prone_factor;
    }
    let idle_scale = iw5_scale_product(assets, |s| s.idle_settings);
    facts.idle.hip_idle_amount_at_0x370 *= idle_scale;
    facts.idle.ads_idle_amount_at_0x36c *= idle_scale;

    if let Some(spread) = iw5_first_block(assets, |a| a.hip_spread) {
        let v = spread.values;
        apply_leftover_hip_spread(
            facts,
            [
                v[0], v[1], v[2], v[3], v[4], v[5], v[9], v[6], v[7], v[8], v[10], v[11],
            ],
        );
    }
    let spread_scale = iw5_scale_product(assets, |s| s.hip_spread);
    if spread_scale != 1.0 {
        for value in [
            &mut facts.hip_spread_stand_min,
            &mut facts.hip_spread_ducked_min,
            &mut facts.hip_spread_prone_min,
            &mut facts.hip_spread_stand_max,
            &mut facts.hip_spread_ducked_max,
            &mut facts.hip_spread_prone_max,
        ] {
            *value *= spread_scale;
        }
    }

    let kick = &mut facts.kick;
    if let Some(gun) = iw5_first_block(assets, |a| a.gun_kick) {
        kick.hip_gun_kick_reduced_kick_bullets = gun.hip_reduced_kick_bullets;
        [
            kick.hip_gun_kick_reduced_kick_percent,
            kick.hip_gun_kick_pitch_min,
            kick.hip_gun_kick_pitch_max,
            kick.hip_gun_kick_yaw_min,
            kick.hip_gun_kick_yaw_max,
            kick.hip_gun_kick_accel,
            kick.hip_gun_kick_speed_max,
            kick.hip_gun_kick_speed_decay,
            kick.hip_gun_kick_static_decay,
        ] = gun.hip;
        kick.ads_gun_kick_reduced_kick_bullets = gun.ads_reduced_kick_bullets;
        [
            kick.ads_gun_kick_reduced_kick_percent,
            kick.ads_gun_kick_pitch_min,
            kick.ads_gun_kick_pitch_max,
            kick.ads_gun_kick_yaw_min,
            kick.ads_gun_kick_yaw_max,
            kick.ads_gun_kick_accel,
            kick.ads_gun_kick_speed_max,
            kick.ads_gun_kick_speed_decay,
            kick.ads_gun_kick_static_decay,
        ] = gun.ads;
    }
    let gun_scale = iw5_scale_product(assets, |s| s.gun_kick);
    if gun_scale != 1.0 {
        for value in [
            &mut kick.hip_gun_kick_pitch_min,
            &mut kick.hip_gun_kick_pitch_max,
            &mut kick.hip_gun_kick_yaw_min,
            &mut kick.hip_gun_kick_yaw_max,
            &mut kick.ads_gun_kick_pitch_min,
            &mut kick.ads_gun_kick_pitch_max,
            &mut kick.ads_gun_kick_yaw_min,
            &mut kick.ads_gun_kick_yaw_max,
        ] {
            *value *= gun_scale;
        }
    }
    if let Some(view) = iw5_first_block(assets, |a| a.view_kick) {
        [
            kick.hip_view_kick_pitch_min,
            kick.hip_view_kick_pitch_max,
            kick.hip_view_kick_yaw_min,
            kick.hip_view_kick_yaw_max,
            kick.f_hip_view_kick_center_speed,
            kick.ads_view_kick_pitch_min,
            kick.ads_view_kick_pitch_max,
            kick.ads_view_kick_yaw_min,
            kick.ads_view_kick_yaw_max,
            kick.f_ads_view_kick_center_speed,
        ] = view;
    }
    let view_scale = iw5_scale_product(assets, |s| s.view_kick);
    if view_scale != 1.0 {
        for value in [
            &mut kick.hip_view_kick_pitch_min,
            &mut kick.hip_view_kick_pitch_max,
            &mut kick.hip_view_kick_yaw_min,
            &mut kick.hip_view_kick_yaw_max,
            &mut kick.ads_view_kick_pitch_min,
            &mut kick.ads_view_kick_pitch_max,
            &mut kick.ads_view_kick_yaw_min,
            &mut kick.ads_view_kick_yaw_max,
        ] {
            *value *= view_scale;
        }
    }
    let center_scale = iw5_scale_product(assets, |s| s.view_center);
    kick.f_hip_view_kick_center_speed *= center_scale;
    kick.f_ads_view_kick_center_speed *= center_scale;

    facts.fire_time_ms = scale_i32(
        facts.fire_time_ms,
        iw5_scale_product(assets, |s| s.fire_timers),
    );
    let state = iw5_scale_product(assets, |s| s.state_timers);
    if state != 1.0 {
        for value in [
            &mut facts.fire_delay_ms,
            &mut facts.melee_delay_ms,
            &mut facts.melee_charge_delay_ms,
            &mut facts.rechamber_time_ms,
            &mut facts.rechamber_bolt_time_ms,
            &mut facts.hold_fire_time_ms,
            &mut facts.melee_time_ms,
            &mut facts.melee_charge_time_ms,
            &mut facts.reload_time_ms,
            &mut facts.reload_show_rocket_time_ms,
            &mut facts.reload_empty_time_ms,
            &mut facts.reload_add_time_ms,
            &mut facts.reload_start_time_ms,
            &mut facts.reload_start_add_time_ms,
            &mut facts.reload_end_time_ms,
            &mut facts.drop_time_ms,
            &mut facts.raise_time_ms,
            &mut facts.quick_drop_time_ms,
            &mut facts.quick_raise_time_ms,
            &mut facts.sprint_raise_time_ms,
            &mut facts.sprint_loop_time_ms,
            &mut facts.sprint_drop_time_ms,
        ] {
            *value = scale_i32(*value, state);
        }
    }
}

fn iw5_anim_timer(facts: &mut WeaponBodyFacts, slot: usize) -> Option<&mut i32> {
    Some(match slot {
        weap_anim::FIRE => &mut facts.fire_time_ms,
        weap_anim::RECHAMBER => &mut facts.rechamber_time_ms,
        weap_anim::MELEE => &mut facts.melee_time_ms,
        weap_anim::MELEE_CHARGE => &mut facts.melee_charge_time_ms,
        weap_anim::RELOAD => &mut facts.reload_time_ms,
        weap_anim::RELOAD_EMPTY => &mut facts.reload_empty_time_ms,
        weap_anim::RELOAD_START => &mut facts.reload_start_time_ms,
        weap_anim::RELOAD_END => &mut facts.reload_end_time_ms,
        weap_anim::RAISE => &mut facts.raise_time_ms,
        weap_anim::DROP => &mut facts.drop_time_ms,
        weap_anim::QUICK_RAISE => &mut facts.quick_raise_time_ms,
        weap_anim::QUICK_DROP => &mut facts.quick_drop_time_ms,
        weap_anim::SPRINT_IN => &mut facts.sprint_raise_time_ms,
        weap_anim::SPRINT_LOOP => &mut facts.sprint_loop_time_ms,
        weap_anim::SPRINT_OUT => &mut facts.sprint_drop_time_ms,
        _ => return None,
    })
}

fn normalize_weapon_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    lower.strip_suffix("_mp").unwrap_or(&lower).to_owned()
}

pub fn gsc_weapon_script_name(catalog_bare: &str) -> String {
    if catalog_bare.is_empty() {
        return String::new();
    }
    if catalog_bare.ends_with("_mp") {
        catalog_bare.to_owned()
    } else {
        format!("{catalog_bare}_mp")
    }
}

impl crate::weapon_families::FamilyContent for WeaponRegistry {
    fn iw5_bind(
        &self,
        base_id: u32,
        attachments: &[String],
    ) -> Result<(), crate::ConfigurationRefusal> {
        let selection = self.resolve_iw5_attachment_slots(base_id, attachments)?;
        self.iw5_primary_attachment_assets(base_id, selection)
            .ok_or_else(|| {
                crate::ConfigurationRefusal::MissingContent(self.name_of(base_id).into())
            })?;
        Ok(())
    }

    fn lookup(&self, namespace: crate::AssetNamespace, name: &str) -> Option<u32> {
        self.by_namespaced
            .get(&(namespace, normalize_weapon_name(name)))
            .copied()
    }

    fn offhand_class(&self, id: u32) -> i32 {
        self.facts_of(id).map_or(0, |facts| facts.offhand_class)
    }

    fn admission(&self, id: u32) -> Result<(), crate::ConfigurationRefusal> {
        self.configuration_admission(id)
    }

    fn prepared(&self, selection: &crate::WeaponSelection) -> Option<u32> {
        self.configurations.get(selection).copied()
    }

    fn prepared_all(&self) -> Vec<(u32, crate::WeaponSelection)> {
        self.configurations
            .iter()
            .map(|(selection, &id)| (id, selection.clone()))
            .collect()
    }

    fn names_in(&self, namespace: crate::AssetNamespace) -> Vec<(u32, String)> {
        (1..=self.len() as u32)
            .filter(|&id| self.namespace_of(id) == Some(namespace))
            .filter(|&id| self.iw5_configuration_of(id).is_none())
            .map(|id| (id, normalize_weapon_name(self.name_of(id))))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FpvAssemblyCensus {
    pub built: usize,
    pub linked: usize,
    pub refused: usize,
    pub clip_tables: usize,
}

fn mint_weapon_revision() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn iw5_best_pair_override<'a, T>(
    rows: &'a [T],
    selection: Iw5AttachmentSelection,
    override_type: u32,
    fields: impl Fn(&T) -> (u16, u16, u32),
) -> Option<&'a T> {
    let candidates = selection.override_candidates();
    let mut best = None;
    let mut best_score = 0;
    for row in rows {
        let (first, second, row_type) = fields(row);
        if row_type != override_type {
            continue;
        }
        let score = u8::from(first != 0 && candidates.contains(&first))
            + u8::from(second != 0 && candidates.contains(&second));
        if score > best_score {
            best = Some(row);
            best_score = score;
        }
    }
    best
}

fn fpv_model_edge(
    hint: Option<&str>,
    ns: crate::AssetNamespace,
    fpv: &crate::FpvMeshCatalog,
) -> AssetEdge<FpvMeshSpace> {
    let hint = hint.filter(|name| !name.is_empty());
    match hint {
        None => AssetEdge::Absent,
        Some(name) => match fpv.index_by_name(ns, name) {
            Some(index) => AssetEdge::bind_order(index, fpv.zone_of(index)),
            None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
        },
    }
}

fn world_model_edge(
    hint: Option<&str>,
    catalog: &crate::WorldWeaponCatalog,
) -> AssetEdge<WorldWeaponSpace> {
    let hint = hint.filter(|name| !name.is_empty());
    match hint {
        None => AssetEdge::Absent,
        Some(name) => match catalog.index_by_name(name) {
            Some(index) => AssetEdge::bind_order(index, catalog.zone_of(index)),
            None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
        },
    }
}

fn sound_alias_in_bank<'a>(
    hint: Option<&str>,
    ns: crate::AssetNamespace,
    catalog: &'a crate::SoundCatalog,
) -> Option<(crate::AssetNamespace, &'a str)> {
    let name = hint.filter(|name| !name.is_empty())?;
    let order = catalog.index_in(ns, name)?;
    Some((catalog.namespace_of_alias(order), catalog.name_at(order)?))
}

fn xanim_hint_edge(
    hint: Option<&str>,
    ns: crate::AssetNamespace,
    xanims: &crate::XAnimCatalog,
) -> AssetEdge<XAnimSpace> {
    let hint = hint.filter(|name| !name.is_empty());
    match hint {
        None => AssetEdge::Absent,
        Some(name) => match xanims.index_by_name(ns, name) {
            Some(index) => AssetEdge::bind_order(index, xanims.zone_of(index)),
            None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
        },
    }
}

fn fx_hint_edge(
    authored_slot: bool,
    hint: Option<&str>,
    fx: &crate::FxCatalog,
) -> AssetEdge<FxSpace> {
    let hint = hint.filter(|name| !name.is_empty());
    if !authored_slot && hint.is_none() {
        return AssetEdge::Absent;
    }
    match hint.and_then(|name| fx.index_by_name(name)) {
        Some(index) => AssetEdge::bind_order(index, fx.zone_of(index)),
        None => AssetEdge::Unresolved(AssetEdgeReason::CatalogMiss),
    }
}

fn merge_sz_xanims(
    dst: &mut [Option<String>; WEAPON_ANIM_SLOTS],
    src: [Option<String>; WEAPON_ANIM_SLOTS],
) {
    for (d, s) in dst.iter_mut().zip(src) {
        if d.is_none() {
            *d = s;
        }
    }
}

fn xanims_idle(names: &[Option<String>; WEAPON_ANIM_SLOTS]) -> Option<&str> {
    names
        .get(weap_anim::IDLE)
        .and_then(|s| s.as_deref())
        .filter(|s| !s.is_empty())
}

fn merge_sound_aliases(dst: &mut WeaponSoundAliases, src: &WeaponSoundAliases) {
    for (dst, src) in [
        (&mut dst.fire, &src.fire),
        (&mut dst.fire_player, &src.fire_player),
        (&mut dst.empty_fire, &src.empty_fire),
        (&mut dst.empty_fire_player, &src.empty_fire_player),
        (&mut dst.melee_swipe, &src.melee_swipe),
        (&mut dst.melee_swipe_player, &src.melee_swipe_player),
        (&mut dst.melee_hit, &src.melee_hit),
        (&mut dst.melee_miss, &src.melee_miss),
        (&mut dst.pickup, &src.pickup),
        (&mut dst.pickup_player, &src.pickup_player),
        (&mut dst.ammo_pickup, &src.ammo_pickup),
        (&mut dst.ammo_pickup_player, &src.ammo_pickup_player),
        (&mut dst.pullback, &src.pullback),
        (&mut dst.pullback_player, &src.pullback_player),
        (&mut dst.reload, &src.reload),
        (&mut dst.reload_player, &src.reload_player),
        (&mut dst.reload_empty, &src.reload_empty),
        (&mut dst.reload_empty_player, &src.reload_empty_player),
        (&mut dst.reload_start, &src.reload_start),
        (&mut dst.reload_start_player, &src.reload_start_player),
        (&mut dst.reload_end, &src.reload_end),
        (&mut dst.reload_end_player, &src.reload_end_player),
        (&mut dst.rechamber, &src.rechamber),
        (&mut dst.rechamber_player, &src.rechamber_player),
        (&mut dst.alt_switch, &src.alt_switch),
        (&mut dst.alt_switch_player, &src.alt_switch_player),
        (&mut dst.raise, &src.raise),
        (&mut dst.raise_player, &src.raise_player),
        (&mut dst.first_raise, &src.first_raise),
        (&mut dst.first_raise_player, &src.first_raise_player),
        (&mut dst.putaway, &src.putaway),
        (&mut dst.putaway_player, &src.putaway_player),
        (&mut dst.proj_explosion, &src.proj_explosion),
        (&mut dst.projectile, &src.projectile),
        (&mut dst.proj_ignition_sound, &src.proj_ignition_sound),
        (&mut dst.fire_player_akimbo, &src.fire_player_akimbo),
        (&mut dst.fire_loop, &src.fire_loop),
        (&mut dst.fire_loop_player, &src.fire_loop_player),
        (&mut dst.fire_stop, &src.fire_stop),
        (&mut dst.fire_stop_player, &src.fire_stop_player),
        (&mut dst.fire_last, &src.fire_last),
        (&mut dst.fire_last_player, &src.fire_last_player),
    ] {
        if dst.is_none() {
            *dst = src.clone();
        }
    }
    for (dst, src) in [
        (&mut dst.fire_ptr_kind, src.fire_ptr_kind),
        (&mut dst.fire_player_ptr_kind, src.fire_player_ptr_kind),
        (&mut dst.reload_player_ptr_kind, src.reload_player_ptr_kind),
    ] {
        if dst.is_none() {
            *dst = src;
        }
    }
    for (dst, src) in dst.bounce.iter_mut().zip(src.bounce.iter()) {
        if dst.is_none() {
            *dst = src.clone();
        }
    }
    if dst.notetrack_sound_map.is_empty() && !src.notetrack_sound_map.is_empty() {
        dst.notetrack_sound_map = src.notetrack_sound_map.clone();
    }
    if dst.notetrack_rumble_map.is_empty() && !src.notetrack_rumble_map.is_empty() {
        dst.notetrack_rumble_map = src.notetrack_rumble_map.clone();
    }
}

fn merge_combat_fx(dst: &mut WeaponCombatFx, src: &WeaponCombatFx) {
    merge_fx_edge(
        &mut dst.view_flash,
        &mut dst.view_flash_hint,
        src.view_flash,
        &src.view_flash_hint,
    );
    merge_fx_edge(
        &mut dst.world_flash,
        &mut dst.world_flash_hint,
        src.world_flash,
        &src.world_flash_hint,
    );
    merge_fx_edge(
        &mut dst.view_shell_eject,
        &mut dst.view_shell_eject_hint,
        src.view_shell_eject,
        &src.view_shell_eject_hint,
    );
    merge_fx_edge(
        &mut dst.world_shell_eject,
        &mut dst.world_shell_eject_hint,
        src.world_shell_eject,
        &src.world_shell_eject_hint,
    );
    merge_fx_edge(
        &mut dst.view_last_shot_eject,
        &mut dst.view_last_shot_eject_hint,
        src.view_last_shot_eject,
        &src.view_last_shot_eject_hint,
    );
    merge_fx_edge(
        &mut dst.world_last_shot_eject,
        &mut dst.world_last_shot_eject_hint,
        src.world_last_shot_eject,
        &src.world_last_shot_eject_hint,
    );
    merge_fx_edge(
        &mut dst.explosion,
        &mut dst.explosion_hint,
        src.explosion,
        &src.explosion_hint,
    );
    if dst.tracer.is_absent() {
        dst.tracer = src.tracer;
        if dst.tracer_hint.is_none() {
            dst.tracer_hint = src.tracer_hint.clone();
        }
    }
    dst.last_shot_eject_pair_authored |= src.last_shot_eject_pair_authored;
}

fn merge_combat_slots(dst: &mut CombatFxSlots, src: &CombatFxSlots) {
    if dst.view_flash.is_none() {
        dst.view_flash = src.view_flash;
    }
    if dst.world_flash.is_none() {
        dst.world_flash = src.world_flash;
    }
    if dst.view_shell_eject.is_none() {
        dst.view_shell_eject = src.view_shell_eject;
    }
    if dst.world_shell_eject.is_none() {
        dst.world_shell_eject = src.world_shell_eject;
    }
    if dst.view_last_shot_eject.is_none() {
        dst.view_last_shot_eject = src.view_last_shot_eject;
    }
    if dst.world_last_shot_eject.is_none() {
        dst.world_last_shot_eject = src.world_last_shot_eject;
    }
    if dst.explosion.is_none() {
        dst.explosion = src.explosion;
    }
    if dst.tracer.is_none() {
        dst.tracer = src.tracer;
    }
}

fn merge_fx_edge(
    dst_edge: &mut AssetEdge<FxSpace>,
    dst_hint: &mut Option<String>,
    src_edge: AssetEdge<FxSpace>,
    src_hint: &Option<String>,
) {
    if dst_edge.is_absent() {
        *dst_edge = src_edge;
        if dst_hint.is_none() {
            *dst_hint = src_hint.clone();
        }
    }
}

fn merge_body_facts(dst: &mut WeaponBodyFacts, src: WeaponBodyFacts) {
    if dst.fire_time_ms == 0 {
        dst.fire_time_ms = src.fire_time_ms;
    }
    if dst.impact_type == 0 && src.impact_type != 0 {
        dst.impact_type = src.impact_type;
    }

    if !dst.body_resolved && src.body_resolved {
        let fire_time_ms = dst.fire_time_ms;
        let ads_zoom_fov = dst.ads_zoom_fov;
        let ads_dof = dst.ads_dof;
        let ads_cs = dst.kick.f_ads_view_kick_center_speed;
        let hip_cs = dst.kick.f_hip_view_kick_center_speed;
        *dst = src;
        dst.fire_time_ms = fire_time_ms;
        dst.ads_zoom_fov = ads_zoom_fov;
        dst.ads_dof = ads_dof;

        dst.kick.f_ads_view_kick_center_speed = ads_cs;
        dst.kick.f_hip_view_kick_center_speed = hip_cs;
        return;
    }

    if dst.raise_time_ms == 0 {
        dst.raise_time_ms = src.raise_time_ms;
    }

    if !kick_body_captured(&dst.kick) && kick_body_captured(&src.kick) {
        let ads_cs = dst.kick.f_ads_view_kick_center_speed;
        let hip_cs = dst.kick.f_hip_view_kick_center_speed;
        dst.kick = src.kick;
        if ads_cs != 0.0 {
            dst.kick.f_ads_view_kick_center_speed = ads_cs;
        }
        if hip_cs != 0.0 {
            dst.kick.f_hip_view_kick_center_speed = hip_cs;
        }
    }
    if dst.kick.f_ads_view_kick_center_speed == 0.0 {
        dst.kick.f_ads_view_kick_center_speed = src.kick.f_ads_view_kick_center_speed;
    }
    if dst.kick.f_hip_view_kick_center_speed == 0.0 {
        dst.kick.f_hip_view_kick_center_speed = src.kick.f_hip_view_kick_center_speed;
    }
    if !sway_body_captured(&dst.sway) && sway_body_captured(&src.sway) {
        dst.sway = src.sway;
    }
    if !stance_ofs_captured(&dst.stance_ofs_at_0x168, &dst.stance_ofs_at_0x18c)
        && stance_ofs_captured(&src.stance_ofs_at_0x168, &src.stance_ofs_at_0x18c)
    {
        dst.stance_ofs_at_0x168 = src.stance_ofs_at_0x168;
        dst.stance_ofs_at_0x18c = src.stance_ofs_at_0x18c;
    }
    if dst.night_vision_wear_time == 0 {
        dst.night_vision_wear_time = src.night_vision_wear_time;
    }

    if !movement_ofs_captured(&dst.movement) && movement_ofs_captured(&src.movement) {
        dst.movement = src.movement;
    }
    if !idle_captured(&dst.idle) && idle_captured(&src.idle) {
        dst.idle = src.idle;
    }
    if dst.select_requires_ammo_at_0x667.is_none() {
        dst.select_requires_ammo_at_0x667 = src.select_requires_ammo_at_0x667;
        dst.quick_drop_time_ms = src.quick_drop_time_ms;
    }
    if dst.offhand_hold_is_cancelable_at_0x681.is_none() {
        dst.offhand_hold_is_cancelable_at_0x681 = src.offhand_hold_is_cancelable_at_0x681;
    }
    if dst.drop_time_ms == 0 {
        dst.drop_time_ms = src.drop_time_ms;
    }
    if dst.offhand_class == 0 && src.offhand_class != 0 {
        dst.offhand_class = src.offhand_class;
    }
    if dst.move_speed_scale == 0.0 {
        dst.move_speed_scale = src.move_speed_scale;
    }
    if dst.ads_move_speed_scale == 0.0 {
        dst.ads_move_speed_scale = src.ads_move_speed_scale;
    }
    if dst.sprint_duration_scale == 0.0 {
        dst.sprint_duration_scale = src.sprint_duration_scale;
    }

    if dst.clip_size == 0 {
        dst.clip_size = src.clip_size;
    }
    if dst.fire_type == 0 && src.fire_type != 0 {
        dst.fire_type = src.fire_type;
    }
    if dst.max_ammo == 0 {
        dst.max_ammo = src.max_ammo;
    }
    if dst.damage == 0 {
        dst.damage = src.damage;
    }
    if dst.rechamber_time_ms == 0 {
        dst.rechamber_time_ms = src.rechamber_time_ms;
    }
    if dst.reload_time_ms == 0 {
        dst.reload_time_ms = src.reload_time_ms;
    }
    if dst.reload_show_rocket_time_ms == 0 {
        dst.reload_show_rocket_time_ms = src.reload_show_rocket_time_ms;
    }
    if dst.reload_empty_time_ms == 0 {
        dst.reload_empty_time_ms = src.reload_empty_time_ms;
    }
    if dst.reload_add_time_ms == 0 {
        dst.reload_add_time_ms = src.reload_add_time_ms;
    }
    if dst.reload_empty_add_time_ms == 0 {
        dst.reload_empty_add_time_ms = src.reload_empty_add_time_ms;
    }
    if dst.reload_start_time_ms == 0 {
        dst.reload_start_time_ms = src.reload_start_time_ms;
    }
    if dst.reload_start_add_time_ms == 0 {
        dst.reload_start_add_time_ms = src.reload_start_add_time_ms;
    }
    if dst.reload_end_time_ms == 0 {
        dst.reload_end_time_ms = src.reload_end_time_ms;
    }
    if dst.reload_ammo_add == 0 {
        dst.reload_ammo_add = src.reload_ammo_add;
    }
    if dst.reload_start_add == 0 {
        dst.reload_start_add = src.reload_start_add;
    }
    if dst.fuse_time_ms == 0 {
        dst.fuse_time_ms = src.fuse_time_ms;
    }
    if dst.sprint_raise_time_ms == 0 {
        dst.sprint_raise_time_ms = src.sprint_raise_time_ms;
    }
    if dst.sprint_loop_time_ms == 0 {
        dst.sprint_loop_time_ms = src.sprint_loop_time_ms;
    }
    if dst.sprint_drop_time_ms == 0 {
        dst.sprint_drop_time_ms = src.sprint_drop_time_ms;
    }
    if dst.hold_fire_time_ms == 0 {
        dst.hold_fire_time_ms = src.hold_fire_time_ms;
    }
    if !dst.cook_off_hold {
        dst.cook_off_hold = src.cook_off_hold;
    }
    if !dst.timed_detonation {
        dst.timed_detonation = src.timed_detonation;
    }
    if !dst.clip_only {
        dst.clip_only = src.clip_only;
    }
    if !dst.proj_impact_explode {
        dst.proj_impact_explode = src.proj_impact_explode;
    }
    if !dst.stick_to_players {
        dst.stick_to_players = src.stick_to_players;
    }
    if !dst.ads_fire_only {
        dst.ads_fire_only = src.ads_fire_only;
    }
    if dst.explosion_radius == 0 {
        dst.explosion_radius = src.explosion_radius;
    }
    if dst.explosion_radius_min == 0 {
        dst.explosion_radius_min = src.explosion_radius_min;
    }
    if dst.explosion_inner_damage == 0 {
        dst.explosion_inner_damage = src.explosion_inner_damage;
    }
    if dst.explosion_outer_damage == 0 {
        dst.explosion_outer_damage = src.explosion_outer_damage;
    }
    if dst.projectile_speed == 0 {
        dst.projectile_speed = src.projectile_speed;
    }
    if dst.projectile_speed_up == 0 {
        dst.projectile_speed_up = src.projectile_speed_up;
    }
    if dst.projectile_speed_forward == 0 {
        dst.projectile_speed_forward = src.projectile_speed_forward;
    }
    if dst.projectile_activate_dist == 0 {
        dst.projectile_activate_dist = src.projectile_activate_dist;
    }
    if dst.projectile_explosion_type == 0 {
        dst.projectile_explosion_type = src.projectile_explosion_type;
    }
    if dst.parallel_bounce.is_none() {
        dst.parallel_bounce = src.parallel_bounce;
    }
    if dst.perpendicular_bounce.is_none() {
        dst.perpendicular_bounce = src.perpendicular_bounce;
    }
    if dst.location_damage_mult.is_none() {
        dst.location_damage_mult = src.location_damage_mult;
    }
    if dst.penetrate_type == 0 && src.penetrate_type != 0 {
        dst.penetrate_type = src.penetrate_type;
    }
    if dst.penetrate_multiplier == 0.0 && src.penetrate_multiplier != 0.0 {
        dst.penetrate_multiplier = src.penetrate_multiplier;
    }
    // motion_tracker belongs to the complete definition, not the shared body.
    if !dst.rifle_bullet && src.rifle_bullet {
        dst.rifle_bullet = true;
    }
    if dst.inventory_type == 0 && src.inventory_type != 0 {
        dst.inventory_type = src.inventory_type;
    }
    if dst.start_ammo == 0 {
        dst.start_ammo = src.start_ammo;
    }
    if !dst.ammo_count_clip_relative && src.ammo_count_clip_relative {
        dst.ammo_count_clip_relative = true;
    }
    if dst.min_damage == 0 {
        dst.min_damage = src.min_damage;
    }
    if dst.min_player_damage == 0 {
        dst.min_player_damage = src.min_player_damage;
    }
    if dst.max_damage_range == 0.0 {
        dst.max_damage_range = src.max_damage_range;
    }
    if dst.min_damage_range == 0.0 {
        dst.min_damage_range = src.min_damage_range;
    }
    if !dst.inherits_perks && src.inherits_perks {
        dst.inherits_perks = true;
    }
    if !dst.no_partial_reload && src.no_partial_reload {
        dst.no_partial_reload = true;
    }
    if !dst.segmented_reload && src.segmented_reload {
        dst.segmented_reload = true;
    }
    if dst.rechamber_bolt_delay_ms == 0 && src.rechamber_bolt_delay_ms != 0 {
        dst.rechamber_bolt_delay_ms = src.rechamber_bolt_delay_ms;
    }
    if !dst.rechamber_while_ads && src.rechamber_while_ads {
        dst.rechamber_while_ads = true;
    }
    if dst.dual_wield_view_model_offset == 0.0 {
        dst.dual_wield_view_model_offset = src.dual_wield_view_model_offset;
    }
    if dst.overlay_interface == 0 {
        dst.overlay_interface = src.overlay_interface;
    }
    if dst.overlay_reticle == 0 {
        dst.overlay_reticle = src.overlay_reticle;
        if dst.ads_overlay_width == 0.0 {
            dst.ads_overlay_width = src.ads_overlay_width;
        }
        if dst.ads_overlay_height == 0.0 {
            dst.ads_overlay_height = src.ads_overlay_height;
        }
    }
    if dst.i_reticle_side_size == 0 {
        dst.i_reticle_side_size = src.i_reticle_side_size;
    }
    if dst.i_reticle_min_ofs == 0 {
        dst.i_reticle_min_ofs = src.i_reticle_min_ofs;
    }
}

fn kick_body_captured(k: &WeaponKickFacts) -> bool {
    k.hip_view_kick_pitch_min != 0.0
        || k.hip_view_kick_pitch_max != 0.0
        || k.ads_view_kick_pitch_min != 0.0
        || k.ads_view_kick_pitch_max != 0.0
        || k.hip_gun_kick_pitch_min != 0.0
        || k.hip_gun_kick_pitch_max != 0.0
        || k.ads_gun_kick_pitch_min != 0.0
        || k.ads_gun_kick_pitch_max != 0.0
        || k.hip_gun_kick_accel != 0.0
        || k.ads_gun_kick_accel != 0.0
}

fn sway_body_captured(s: &WeaponSwayFacts) -> bool {
    s.sway_max_angle != 0.0
        || s.sway_lerp_speed != 0.0
        || s.sway_pitch_scale != 0.0
        || s.sway_yaw_scale != 0.0
        || s.ads_sway_max_angle != 0.0
        || s.ads_sway_lerp_speed != 0.0
}

fn stance_ofs_captured(duck: &[f32; 3], prone: &[f32; 3]) -> bool {
    duck.iter().any(|v| *v != 0.0) || prone.iter().any(|v| *v != 0.0)
}

fn movement_ofs_captured(m: &WeaponMovementOfsInputs) -> bool {
    m.stand_move_at_0x138.iter().any(|v| *v != 0.0)
        || m.stand_rot_at_0x144.iter().any(|v| *v != 0.0)
        || m.strafe_move_at_0x150.iter().any(|v| *v != 0.0)
        || m.strafe_rot_at_0x15c.iter().any(|v| *v != 0.0)
        || m.ducked_move_at_0x174.iter().any(|v| *v != 0.0)
        || m.ducked_rot_at_0x180.iter().any(|v| *v != 0.0)
        || m.prone_move_at_0x198.iter().any(|v| *v != 0.0)
        || m.prone_rot_at_0x1a4.iter().any(|v| *v != 0.0)
        || m.pos_move_rate_at_0x1b0 != 0.0
        || m.pos_prone_move_rate_at_0x1b4 != 0.0
        || m.stand_move_min_speed_at_0x1b8 != 0.0
        || m.ducked_move_min_speed_at_0x1bc != 0.0
        || m.prone_move_min_speed_at_0x1c0 != 0.0
        || m.pos_rot_rate_at_0x1c4 != 0.0
        || m.pos_prone_rot_rate_at_0x1c8 != 0.0
}

fn idle_captured(i: &WeaponIdleInputs) -> bool {
    i.ads_idle_amount_at_0x36c != 0.0
        || i.hip_idle_amount_at_0x370 != 0.0
        || i.ads_idle_speed_at_0x374 != 0.0
        || i.hip_idle_speed_at_0x378 != 0.0
        || i.idle_crouch_factor_at_0x37c != 0.0
        || i.idle_prone_factor_at_0x380 != 0.0
}
