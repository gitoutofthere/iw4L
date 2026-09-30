use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Instant;

use bevy::core_pipeline::core_3d::{CORE_3D_DEPTH_FORMAT, main_opaque_pass_3d};
use bevy::core_pipeline::upscaling::ViewUpscalingPipeline;
use bevy::core_pipeline::{Core3d, Core3dSystems};
use bevy::mesh::VertexBufferLayout;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::diagnostic::RecordDiagnostics;
use bevy::render::render_phase::TrackedRenderPass;
use bevy::render::render_resource::binding_types::{
    sampler, storage_buffer_read_only, texture_2d, texture_3d, texture_cube, uniform_buffer_sized,
};
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindingResource, Buffer, BufferBinding,
    BufferDescriptor, BufferInitDescriptor, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState, Face, FrontFace,
    IndexFormat, LoadOp, MultisampleState, Operations, PipelineCache, PolygonMode, PrimitiveState,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    SamplerBindingType, ShaderStages, SpecializedRenderPipelines, StoreOp, TextureFormat,
    TextureSampleType, TextureView, TextureViewDescriptor, VertexAttribute, VertexFormat,
    VertexStepMode,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::view::{ExtractedView, Msaa, ViewDepthTexture, ViewTarget};
use bevy::render::{Render, RenderSystems};

use super::ExtractedRenderFrameProducts;
use super::backend::{
    ModelIndexRingCopyRefuse, ModelIndexStream, PackDraw, PackedEmit, PackedListKind,
    SmodelRigidFlush, pack_sun_shadow_frontend, prim_args_from_world_flush,
    prim_args_u32_index_span, r_draw_spot_shadow_map, r_draw_sun_shadow_map_forced,
    r_draw_surf_list_work_colour,
};
use super::depth_range::{
    GFX_DEPTH_RANGE_VIEWMODEL, depth_range_type_for_draw, reverse_z_viewport_depth,
};
use super::exact_pipeline::{
    ExactModuleSource, ExactPipelinePlan, ExactPipelineRegistry, ExactPipelineSlot,
};
use super::floatz::{self, ExactFloatZResolve};
use super::gpu_contract::{
    WgpuBindLayoutEntry, WgpuBindingKind, WgpuShaderVisibility, WgpuVertexFormat,
};
use super::gpu_prepare::{
    ConstantPackRefusal, PassConstantBuffers, overlay_packed_code_on_banks, split_bind_layout,
};
use super::gpu_resources::{
    RetailSamplerTable, RuntimeProgramPortGpuExt, RuntimeUploadedImageRegistry, TextureBindRefusal,
    UploadedTextureBind, padded_upload_len, write_buffer_padded, write_buffer_range,
};
use super::shadowmap_spot_gpu::{
    SHADOWMAP_SPOT_COLOR_FORMAT, SHADOWMAP_SPOT_DEPTH_FORMAT, ShadowmapSpotGpu,
    ensure_shadowmap_spot_targets,
};
use super::shadowmap_sun_gpu::{
    SHADOWMAP_SUN_COLOR_FORMAT, SHADOWMAP_SUN_DEPTH_FORMAT, ShadowmapSunGpu,
    ensure_shadowmap_sun_target, scissor_xywh,
};
use super::sm3_wgsl::{PASS_FRAGMENT_ENTRY, alpha_test_fragment_entry};
use super::smodel_cache_gpu::SmodelCacheGpu;
use super::smodel_cached::{
    cached_lighting_location, cached_lighting_vertex_buffers, inject_cached_lighting_attribute,
    lighting_texcoord_register,
};
use super::state::{ChangeState0Host, ChangeState1Host, GfxPassState};
use super::texture_table::{
    self, ExactTextureTable, SceneTextureTables, ShadowTextureTable, TableEpoch,
    TextureTableRefusal,
};
use crate::diag::render_frame_diag::{
    GPU_SPAN_COLOUR, GPU_SPAN_EMISSIVE, GPU_SPAN_FLOATZ, GPU_SPAN_SPOT, GPU_SPAN_SUN,
};
use asset_iw4::{D3DCMP_ALWAYS, D3DCMP_EQUAL, D3DCMP_LESS, D3DCMP_LESSEQUAL};
use render_backend::{MaterialExecView, MaterialRunExecutor, PlaceLanes};
use render_frame::MaterialExecFrame;
use render_frame::{BspCameraLane, RetainedDrawItem, RetainedDrawKind, SmodelPretessRange};
use render_frame::{CODE_MESH_INDEX_CAP, CODE_MESH_VERT_CAP, CODE_MESH_VERT_STRIDE};
use render_frame::{
    FrameProduct, FrameProductKind, FrameProductStatus, PackedFrontendLists, SunShadowForcedFrame,
    SunShadowPartition, SunShadowViewport, TextureBindIdentity,
};
use render_material::{
    ExecutablePassView, MaterialExecution, MaterialGenerationId, MaterialRefusal,
    PackedCodeConstantLane, PackedCodeConstants, PortId, PreparedMaterialTable, RuntimeCodeSources,
    RuntimeMaterialCatalog, RuntimeShaderPair, RuntimeShaderStage, RuntimeSortedMaterialTable,
    TechType, prepared_draw_technique,
};

pub const CODE_TEXTURE_FLOATZ: u32 = 0x0f;

pub const CODE_TEXTURE_RESOLVED_POST_SUN: u32 = 9;

pub const CODE_TEXTURE_SHADOWMAP_SUN: u32 = 6;

pub const CODE_TEXTURE_SHADOWMAP_SPOT: u32 = 7;

/// Geometry that survives frame replacement within a world generation.
#[derive(Clone, Debug, Default)]
pub struct ExtractedStaticGeometry {
    pub world_vertices: Arc<Vec<[u8; asset_iw4::size::GFX_WORLD_VERTEX]>>,
    pub world_layer: Arc<Vec<u8>>,
    pub world_indices: Arc<Vec<u32>>,
    pub world_surface_ranges: Arc<Vec<(u32, u32)>>,
    pub world_vertex_refusal: Option<render_frame::RetailWorldVertexRefusal>,
    pub smodel_vertices: Arc<Vec<[u8; asset_iw4::size::GFX_PACKED_VERTEX]>>,
    pub smodel_indices: Arc<Vec<u32>>,
    pub smodel_surface_ranges: Arc<Vec<(u32, u32)>>,
    pub smodel_vertex_refusal: Option<render_frame::RetailPackedVertexRefusal>,
    pub smodel_cached_vertices: Arc<Vec<[u8; asset_iw4::size::GFX_PACKED_VERTEX]>>,
    pub smodel_surface_verts: Arc<Vec<(u32, u32)>>,
}

/// Installed world and material resources. Longer-lived than a colour frame:
/// static geometry, ports, catalog and prepared tables stay here.
#[derive(Resource, Clone, Debug, Default)]
pub struct InstalledRenderWorld(Arc<RenderWorldData>);

impl InstalledRenderWorld {
    pub fn new(data: RenderWorldData) -> Self {
        Self(Arc::new(data))
    }
}

impl std::ops::Deref for InstalledRenderWorld {
    type Target = RenderWorldData;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Consumed by publication; no mutable access through an installed handle.
#[derive(Clone, Debug, Default)]
pub struct RenderWorldData {
    pub generation: MaterialGenerationId,
    pub world_generation: frame::WorldGeneration,
    pub world_products: frame::WorldProducts,
    pub smc_revision: Option<u64>,
    pub ports: Arc<Vec<super::AdmittedExactPort>>,
    pub static_geometry: Arc<ExtractedStaticGeometry>,
    pub smodel_pretess_indices: std::sync::Arc<Vec<u16>>,
    pub smodel_index_layout_revision: u64,
    pub smc_index_baked: Arc<Vec<u16>>,
    pub sampler_table: Option<RetailSamplerTable>,
    pub image_handles: super::gpu_resources::RuntimeImageHandles,
    pub catalog: Option<Arc<RuntimeMaterialCatalog>>,
    pub prepared: Option<Arc<PreparedMaterialTable>>,
    pub sorted_material_names: Arc<Vec<String>>,
    pub shader_program_names: Arc<Vec<Option<String>>>,
    pub sun_effects: Option<render_frame::SunEffectsDef>,
}

/// Commands and changed data of the current frame. Names the installed world by
/// generation instead of carrying world/material tables again.
#[derive(Resource, Clone, Debug, Default)]
pub struct PublishedRenderFrame {
    world: InstalledRenderWorld,
    data: Arc<RenderFrameData>,
}

impl PublishedRenderFrame {
    pub fn seal(world: InstalledRenderWorld, data: RenderFrameData) -> Self {
        assert_eq!(world.generation, data.generation);
        assert_eq!(world.world_generation, data.world_generation);
        Self {
            world,
            data: Arc::new(data),
        }
    }
    pub fn world(&self) -> &InstalledRenderWorld {
        &self.world
    }
}

impl std::ops::Deref for PublishedRenderFrame {
    type Target = RenderFrameData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderFrameData {
    pub frame_products: ExtractedRenderFrameProducts,
    pub generation: MaterialGenerationId,
    pub world_generation: frame::WorldGeneration,
    pub sun_shadow: Option<SunShadowForcedFrame>,
    pub sun_effects: Option<render_frame::SunEffectsFrame>,
    pub warm_pipelines: bool,
    pub pipeline_world_materials: Arc<std::collections::HashSet<u16>>,
    pub pipeline_smodel_materials: Arc<std::collections::HashSet<u16>>,
    pub pipeline_demand_revision: u64,
    pub smc_vb_patches: Vec<(lighting_iw4::SmcPatchLock, Vec<u8>)>,
    pub smc_ib_patches: Vec<(u32, Vec<u8>)>,
    pub xmodel_vertices: Arc<Vec<[u8; asset_iw4::size::GFX_PACKED_VERTEX]>>,
    pub xmodel_indices: Arc<Vec<u32>>,
    pub xmodel_surface_ranges: Arc<Vec<(u32, u32)>>,
    pub xmodel_vertex_refusal: Option<render_frame::RetailPackedVertexRefusal>,
    pub fx_vertices: Arc<Vec<[u8; asset_iw4::size::GFX_PACKED_VERTEX]>>,
    pub fx_indices: Arc<Vec<u32>>,
    pub fx_surface_ranges: Arc<Vec<(u32, u32)>>,
    pub fx_vertex_refusal: Option<render_frame::RetailPackedVertexRefusal>,
    pub fx_revision: u64,
    pub xmodel_revision: u64,
    pub xmodel_topology_revision: u64,
    pub xmodel_packed_segments: render_frame::PackedSegments,
    pub particle_cloud_vertices: Arc<Vec<[u8; fx_iw4::GFX_POS_TEX_VERTEX_STRIDE]>>,
    pub particle_cloud_indices: Arc<Vec<u32>>,
    pub particle_cloud_surface_ranges: Arc<Vec<(u32, u32)>>,
    pub mark_mesh_vertices: Arc<Vec<[u8; asset_iw4::size::GFX_WORLD_VERTEX]>>,
    pub mark_mesh_indices: Arc<Vec<u16>>,
    pub mark_mesh_surface_ranges: Arc<Vec<(u32, u32)>>,
    pub mark_mesh_revision: u64,
    pub glass_mesh_vertices: Arc<Vec<[u8; asset_iw4::size::GFX_PACKED_VERTEX]>>,
    pub glass_mesh_indices: Arc<Vec<u32>>,
    pub glass_mesh_surface_ranges: Arc<Vec<(u32, u32)>>,
    pub glass_mesh_revision: u64,
    pub glass_mesh_vertex_refusal: Option<render_frame::RetailPackedVertexRefusal>,
    pub exec_frame: MaterialExecFrame,
}

/// Borrowed view of the two extract resources. Frame does not own world tables.
#[derive(Clone, Copy)]
struct ExtractedColourRefs<'a> {
    world: &'a InstalledRenderWorld,
    frame: &'a PublishedRenderFrame,
}

impl<'a> ExtractedColourRefs<'a> {
    fn new(world: &'a InstalledRenderWorld, frame: &'a PublishedRenderFrame) -> Self {
        Self { world, frame }
    }
}

mod arena;
mod geometry;
mod indirect;
mod pipeline;
mod prepare_camera;
mod record;
mod residency;
mod shadowmap;
mod smodel_skinned;

use shadowmap::{
    PreparedSpotWork, PreparedSunWork, ResidentShadowStaticDraws, SunFlushKind,
    prepare_shadowmap_spot, prepare_shadowmap_sun, record_shadowmap_spot, record_shadowmap_sun,
};

use arena::{
    ArenaDirty, ArenaPack, append_constant_span, texture_table_layout, upload_constant_arena,
    upload_packed_arena,
};

pub use pipeline::cached_lighting_port_variant;

#[derive(Resource, Default)]
struct ExactColourGeometry {
    generation: MaterialGenerationId,
    world_generation: frame::WorldGeneration,
    world_products: frame::WorldProducts,
    world_vertex: Option<Buffer>,
    world_layer: Option<Buffer>,
    world_index: Option<Buffer>,
    world_surface_ranges: Vec<(u32, u32)>,
    world_vertex_count: usize,
    world_layer_count: usize,
    world_index_count: usize,

    world_cpu_indices: Vec<u32>,
    smodel_vertex: Option<Buffer>,
    smodel_index: Option<Buffer>,
    smodel_surface_ranges: Vec<(u32, u32)>,
    smodel_vertex_count: usize,
    smodel_index_count: usize,
    smodel_cached_vertex: Option<Buffer>,
    smodel_cached_index: Option<Buffer>,
    smodel_cached_vertex_count: usize,

    xmodel: residency::GpuMesh,
    xmodel_surface_ranges: Vec<(u32, u32)>,

    xmodel_resident_segments: render_frame::PackedSegments,

    xmodel_resident_allocation: u64,
    fx_vertex: Option<Buffer>,
    fx_index: Option<Buffer>,
    fx_surface_ranges: Vec<(u32, u32)>,
    fx_vertex_count: usize,
    fx_index_count: usize,
    fx_revision: u64,

    fx_copy_dst: bool,
    particle_cloud_vertex: Option<Buffer>,
    particle_cloud_index: Option<Buffer>,
    particle_cloud_surface_ranges: Vec<(u32, u32)>,
    mark_mesh: residency::GpuMesh,
    mark_mesh_surface_ranges: Vec<(u32, u32)>,
    glass_mesh: residency::GpuMesh,
    glass_mesh_surface_ranges: Vec<(u32, u32)>,
    last_xmodel_gpu_hash: Option<i64>,
}

#[derive(Resource, Default)]
struct ExactColourPipeline {
    generation: MaterialGenerationId,
    ports: Vec<ExactColourPortGpu>,
    by_id: HashMap<PortId, usize>,
}

impl ExactColourPipeline {
    fn get(&self, id: PortId) -> Option<&ExactColourPortGpu> {
        self.by_id.get(&id).map(|&index| &self.ports[index])
    }

    fn rebuild_index(&mut self) {
        self.by_id.clear();
        self.by_id.reserve(self.ports.len());
        for (index, port) in self.ports.iter().enumerate() {
            self.by_id.insert(port.port.id(), index);
        }
    }
}

struct ExactColourPortGpu {
    constants_layout: BindGroupLayoutDescriptor,
    textures_layout: BindGroupLayoutDescriptor,
    vertex_buffers: Vec<VertexBufferLayout>,
    cached_source: Option<Arc<str>>,
    cached_vertex_buffers: Vec<VertexBufferLayout>,
    port: super::AdmittedExactPort,
}

#[derive(Resource, Default)]
struct ExactColourBindingCache {
    generation: MaterialGenerationId,
    views_revision: u64,
    textures: [HashMap<BoundTextureKey, Arc<[u32]>>; 4],
}

#[derive(Resource, Default)]
struct ExactShadowBindingCache {
    generation: MaterialGenerationId,
    views_revision: u64,
    textures: HashMap<BoundTextureKey, Arc<[u32]>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BoundTextureKey {
    identity: TextureBindIdentity,
    spot_shadow_select: Option<u8>,
}

impl ExactColourBindingCache {
    fn open_epoch(&mut self, generation: MaterialGenerationId, views_revision: u64) {
        if self.generation == generation && self.views_revision == views_revision {
            return;
        }
        self.generation = generation;
        self.views_revision = views_revision;
        for slots in &mut self.textures {
            slots.clear();
        }
    }

    fn interned_n(&self) -> usize {
        self.textures.iter().map(HashMap::len).sum()
    }
}

impl ExactShadowBindingCache {
    fn open_epoch(&mut self, generation: MaterialGenerationId, views_revision: u64) {
        if self.generation == generation && self.views_revision == views_revision {
            return;
        }
        self.generation = generation;
        self.views_revision = views_revision;
        self.textures.clear();
    }

    fn clear_textures(&mut self) {
        self.textures.clear();
    }
}

fn open_scene_table_epoch(
    binding_cache: &mut ExactColourBindingCache,
    scene_tables: &mut SceneTextureTables,
    scratch: &mut ColourSubmitScratch,
    uploaded: &RuntimeUploadedImageRegistry,
    generation: MaterialGenerationId,
) {
    let epoch = TableEpoch {
        generation,
        replaced_revision: uploaded.replaced_revision(),
    };
    for table in &mut scene_tables.0 {
        table.open_epoch(epoch);
    }
    scratch.prepared_scene_epoch = epoch;
    binding_cache.open_epoch(generation, uploaded.views_revision());
}

fn open_shadow_table_epoch(
    binding_cache: &mut ExactShadowBindingCache,
    shadow_table: &mut ShadowTextureTable,
    scratch: &mut ShadowSubmitScratch,
    uploaded: &RuntimeUploadedImageRegistry,
    generation: MaterialGenerationId,
) {
    let epoch = TableEpoch {
        generation,
        replaced_revision: uploaded.replaced_revision(),
    };
    shadow_table.open_epoch(epoch);
    scratch.prepared_shadow_epoch = epoch;
    binding_cache.open_epoch(generation, uploaded.views_revision());
}

#[derive(Resource, Default)]
struct ExactPipelineKickCache {
    current: Vec<ExactPipelineSlot>,
    scheduled: HashSet<ExactColourPipelineKey>,
    generation: Option<MaterialGenerationId>,
    views: Vec<(TextureFormat, u32)>,
    demand_revision: u64,
}

#[derive(Default)]
struct GpuConstantArena {
    buffer: Option<Buffer>,
    capacity: u64,
    bind_group: Option<BindGroup>,

    uploaded: Vec<u8>,

    dirty_scratch: Vec<(usize, usize)>,
}

#[derive(Resource, Default)]
struct ExactConstantArena {
    generation: MaterialGenerationId,
    gpu: GpuConstantArena,
}

#[derive(Resource, Default)]
struct ShadowmapSunArena {
    generation: MaterialGenerationId,
    gpu: [GpuConstantArena; super::SUN_SHADOW_PARTITION_COUNT as usize],
}

#[derive(Resource, Default)]
struct ShadowmapSpotArena {
    generation: MaterialGenerationId,
    gpu: GpuConstantArena,

    pack: ArenaPack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ExactColourPipelineKey {
    target: TextureFormat,
    depth_format: TextureFormat,
    samples: u32,
    state0: ChangeState0Host,
    state1: ChangeState1Host,

    pub(super) port: PortId,

    cached_lighting: bool,

    mark_mesh: bool,

    forward_z: bool,
}

fn exact_colour_target_format(target: TextureFormat, srgb_write: bool) -> TextureFormat {
    if srgb_write {
        target.add_srgb_suffix()
    } else {
        target
    }
}

struct ExactColourTargetViews {
    main_raw: TextureView,
    main_srgb: TextureView,
    sampled_raw: Option<TextureView>,
    sampled_srgb: Option<TextureView>,
}

impl ExactColourTargetViews {
    fn new(target: &ViewTarget) -> Self {
        let raw_format = target.main_texture_format();
        let srgb_format = raw_format.add_srgb_suffix();
        Self {
            main_raw: target.main_texture_view().clone(),
            main_srgb: target.main_texture().create_view(&TextureViewDescriptor {
                label: Some("iw4_exact_colour_main_srgb"),
                format: Some(srgb_format),
                ..Default::default()
            }),
            sampled_raw: target.sampled_main_texture_view().cloned(),
            sampled_srgb: target.sampled_main_texture().map(|texture| {
                texture.create_view(&TextureViewDescriptor {
                    label: Some("iw4_exact_colour_sampled_srgb"),
                    format: Some(srgb_format),
                    ..Default::default()
                })
            }),
        }
    }

    fn attachment_views(&self, srgb_write: bool) -> (&TextureView, Option<&TextureView>) {
        let (main, sampled) = if srgb_write {
            (&self.main_srgb, self.sampled_srgb.as_ref())
        } else {
            (&self.main_raw, self.sampled_raw.as_ref())
        };
        match sampled {
            Some(sampled) => (sampled, Some(main)),
            None => (main, None),
        }
    }
}

fn exact_fragment_entry(
    target: TextureFormat,
    forward_z: bool,
    alpha_test: Option<d3d9_state::AlphaTest>,
) -> String {
    if forward_z && target == TextureFormat::R32Float {
        PASS_FRAGMENT_ENTRY.to_owned()
    } else {
        alpha_test_fragment_entry(alpha_test)
    }
}

fn exact_pipeline_plan(
    ports: &ExactColourPipeline,
    registry: &ExactPipelineRegistry,
    device: &RenderDevice,
    key: ExactColourPipelineKey,
) -> (ExactModuleSource, ExactPipelinePlan) {
    let port = ports
        .get(key.port)
        .expect("ExactColourPipelineKey.port is an admitted PortId");
    let depth_compare = if key.state1.depth_test_enable {
        Some(if key.forward_z {
            d3d_zfunc(key.state1.depth_func)
        } else {
            d3d_zfunc_to_reverse_z(key.state1.depth_func)
        })
    } else {
        Some(CompareFunction::Always)
    };
    let cached_lighting = key.cached_lighting && port.cached_source.is_some();
    let source = match (cached_lighting, port.cached_source.as_ref()) {
        (true, Some(cached)) => ExactModuleSource::CachedLighting(cached.clone()),
        _ => ExactModuleSource::Port(port.port.shared_module()),
    };
    let plan = ExactPipelinePlan {
        label: format!(
            "iw4_exact_colour/{:016x}/{}",
            key.port.vertex_program_hash, key.port.vertex_type
        ),
        fragment_entry: exact_fragment_entry(key.target, key.forward_z, key.state0.alpha_test),
        vertex_buffers: if cached_lighting && !port.cached_vertex_buffers.is_empty() {
            port.cached_vertex_buffers.clone()
        } else {
            let mut buffers = port.vertex_buffers.clone();

            if key.mark_mesh
                && let Some(buffer) = buffers.first_mut()
            {
                buffer.array_stride = asset_iw4::size::GFX_WORLD_VERTEX as u64;
            }
            buffers
        },
        targets: vec![Some(ColorTargetState {
            format: key.target,

            blend: if key.target == TextureFormat::R32Float {
                None
            } else {
                key.state0.blend.blend_state()
            },
            write_mask: if key.target == TextureFormat::R32Float {
                key.state0.colour_writes() & ColorWrites::RED
            } else {
                key.state0.colour_writes()
            },
        })],
        primitive: exact_primitive_state(key.state0.cull, key.state0.line_fill),
        depth_stencil: DepthStencilState {
            format: key.depth_format,
            depth_write_enabled: Some(key.state1.depth_write),
            depth_compare,
            stencil: Default::default(),
            bias: if key.forward_z {
                polygon_offset_bias_forward_z(key.state1.polyoffset_level)
            } else {
                polygon_offset_bias(key.state1.polyoffset_level)
            },
        },
        multisample: MultisampleState {
            count: key.samples,
            ..Default::default()
        },
        constants_layout: registry.bind_group_layout(device, &port.constants_layout),
        textures_layout: registry.bind_group_layout(device, &port.textures_layout),
    };
    (source, plan)
}

fn request_exact_pipeline(
    registry: &mut ExactPipelineRegistry,
    ports: &ExactColourPipeline,
    device: &RenderDevice,
    key: ExactColourPipelineKey,
) -> ExactPipelineSlot {
    if let Some(slot) = registry.slot(&key) {
        return slot;
    }
    let (source, plan) = exact_pipeline_plan(ports, registry, device, key);
    registry.request(key, source, plan)
}

fn d3d_zfunc(depth_func_index: u8) -> CompareFunction {
    let d3d = asset_iw4::S_DEPTH_TEST_TABLE
        .get(usize::from(depth_func_index))
        .copied()
        .unwrap_or(D3DCMP_LESSEQUAL);
    match d3d {
        D3DCMP_ALWAYS => CompareFunction::Always,
        D3DCMP_LESS => CompareFunction::Less,
        D3DCMP_EQUAL => CompareFunction::Equal,
        D3DCMP_LESSEQUAL => CompareFunction::LessEqual,
        _ => CompareFunction::LessEqual,
    }
}

fn d3d_zfunc_to_reverse_z(depth_func_index: u8) -> CompareFunction {
    let d3d = asset_iw4::S_DEPTH_TEST_TABLE
        .get(usize::from(depth_func_index))
        .copied()
        .unwrap_or(D3DCMP_LESSEQUAL);
    match d3d {
        D3DCMP_ALWAYS => CompareFunction::Always,
        D3DCMP_LESS => CompareFunction::Greater,
        D3DCMP_EQUAL => CompareFunction::Equal,
        D3DCMP_LESSEQUAL => CompareFunction::GreaterEqual,
        _ => CompareFunction::GreaterEqual,
    }
}

pub(super) fn exact_primitive_state(cull: u8, line_fill: bool) -> PrimitiveState {
    PrimitiveState {
        front_face: FrontFace::Cw,
        cull_mode: match cull {
            1 => Some(Face::Back),
            2 => Some(Face::Front),
            _ => None,
        },
        polygon_mode: if line_fill {
            PolygonMode::Line
        } else {
            PolygonMode::Fill
        },
        ..Default::default()
    }
}

fn polygon_offset_bias(level: u8) -> DepthBiasState {
    let (slope_scale, constant) = asset_iw4::polygon_offset_wgpu_defaults(u32::from(level));
    DepthBiasState {
        constant: -constant,
        slope_scale: -slope_scale,
        clamp: 0.0,
    }
}

fn polygon_offset_bias_forward_z(level: u8) -> DepthBiasState {
    let (slope_scale, constant) = asset_iw4::polygon_offset_wgpu_defaults(u32::from(level));
    DepthBiasState {
        constant,
        slope_scale,
        clamp: 0.0,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GpuSubmitRefusal {
    NoExactPort {
        pair: RuntimeShaderPair,
    },
    ConstantPack(ConstantPackRefusal),
    TextureBind(TextureBindRefusal),
    TextureTable(TextureTableRefusal),
    ProductDependencyNotReady {
        product: FrameProductKind,
    },
    UnsupportedTessKind,

    MultipleCameraViews {
        views: u32,
    },
    WorldVertex(render_frame::RetailWorldVertexRefusal),
    PackedVertex(render_frame::RetailPackedVertexRefusal),
    MissingSurfaceRange {
        surf: u16,
    },
    MissingSmodelRange {
        surface: u32,
    },
    MissingXModelRange {
        surface: u32,
    },
    MissingCodeMeshRange {
        draw: u32,
    },
    MissingParticleCloudRange {
        draw: u32,
    },
    MissingMarkMeshRange {
        draw: u32,
    },
    MissingGlassMeshRange {
        draw: u32,
    },
    EmptyIndexRange {
        surf: u16,
    },
    EmptySmodelIndexRange {
        surface: u32,
    },
    EmptyXModelIndexRange {
        surface: u32,
    },
    EmptyCodeMeshIndexRange {
        draw: u32,
    },
    EmptyParticleCloudIndexRange {
        draw: u32,
    },
    EmptyMarkMeshIndexRange {
        draw: u32,
    },
    EmptyGlassMeshIndexRange {
        draw: u32,
    },
    PipelineNotReady,
    SmodelCacheIndexEmpty {
        placement: u32,
    },
    SmodelCacheIndicesMissing {
        cache_index: u16,
    },
    SmodelXSurfacePathUnread {
        placement: u32,
    },

    SmodelCachedWithoutDestRange {
        placement: u32,
    },
    SmodelSkinnedDestMissing {
        placement: u32,
    },
    ConstantArenaMissing,

    WorldPretessSpanBeyondLimit {
        start: u32,
        count: u32,
        logical_len: u32,
        epoch: u64,
    },

    WorldPretessEpochMismatch {
        span_epoch: u64,
        layout_epoch: u64,
    },
}

#[derive(Resource, Default)]
pub(super) struct ExactColourSubmitCensus {
    ready_draws: u32,
    refused_draws: u32,
    log_frame: u32,

    pub submitted_keys: Vec<u64>,

    pub world_exec_ready_keys: Vec<u64>,

    pub bsp_submitted_surfaces: [u32; 4],

    pub bsp_submit_refused_surfaces: [u32; 4],

    pub bsp_drawn_surfaces: [u32; 4],

    pub bsp_draw_refused_surfaces: [u32; 4],
    submit_prepare_ms: Option<f32>,

    colour_submit_ms: Option<f32>,

    submit_encode_ms: Option<f32>,

    submit_gather_ms: Option<f32>,

    pass_end_ms: Option<f32>,

    encoder_finish_ms: Option<f32>,

    submit_arena_ms: Option<f32>,

    submit_record_ms: Option<f32>,

    pack_intern_hit_n: Option<u32>,

    pack_intern_miss_n: Option<u32>,

    pack_arena_share_n: Option<u32>,

    gpu_exec_reuse_n: Option<u32>,

    gpu_exec_unique_n: Option<u32>,

    pack_overlay_n: Option<u32>,

    pack_overlay_row_n: Option<u32>,

    pack_overlay_pixel_share_n: Option<u32>,

    pack_arena_vertex_n: Option<u32>,

    pack_arena_pixel_n: Option<u32>,

    pack_seed_n: Option<u32>,

    pack_walk_n: Option<u32>,

    tex_bind_hit_n: Option<u32>,

    tex_bind_miss_n: Option<u32>,

    markmesh_hits: Option<u32>,

    markmesh_prepared: Option<u32>,

    last_markmesh_refusal: Option<String>,

    last_markmesh_exec_skip: Option<String>,

    markmesh_missing_58: Option<u32>,

    last_mark_packed_custom: Option<u8>,

    last_mark_packed_scene_light: Option<u8>,

    last_mark_lmap_sampler: Option<u32>,
    glassmesh_hits: Option<u32>,
    glassmesh_prepared: Option<u32>,
    last_glassmesh_exec_skip: Option<String>,
    last_glassmesh_refusal: Option<String>,
    last_glass_packed_probe: Option<u8>,
    last_glass_probe_sampler: Option<u32>,

    gpu_ready: Option<u32>,

    pub set_bind_group0_n: Option<u32>,

    pub set_bind_group1_n: Option<u32>,

    pub material_runs_n: Option<u32>,

    pub pass_setups_n: Option<u32>,

    pub obj_binds_n: Option<u32>,

    pub shell_hits_n: Option<u32>,

    pub shell_misses_n: Option<u32>,

    pub overlay_const_writes_n: Option<u32>,

    pub overlay_need_known_n: Option<u32>,

    set_bind_group_n: Option<u32>,

    pub set_state_n: Option<u32>,

    pub multi_draw_n: Option<u32>,
    pub multi_draw_commands_n: Option<u32>,

    gpu_prepared: Option<u32>,

    gpu_world_ready: Option<u32>,

    gpu_smodel_ready: Option<u32>,

    gpu_xmodel_ready: Option<u32>,

    end_depth_restore_n: Option<u32>,

    end_depth_range_type: Option<i32>,

    code_mesh_gpu_kind: Option<i32>,

    sun_shadow_gpu: Option<u32>,

    sun_shadow_gpu_miss: Option<u32>,

    sun_shadow_gpu_cause: Option<String>,

    sun_shadow_gpu_causes: Option<String>,

    spot_shadow_gpu: Option<u32>,
    spot_shadow_gpu_miss: Option<u32>,
    spot_shadow_gpu_cause: Option<String>,
    spot_shadow_slot_n: Option<u32>,

    sun_shadow_submit_ms: Option<f32>,

    sun_shadow_prepare_ms: Option<f32>,

    sun_shadow_patch_ms: Option<f32>,

    sun_shadow_arena_ms: Option<f32>,

    sun_shadow_record_ms: Option<f32>,

    sun_shadow_finish_ms: Option<f32>,

    sun_shadow_queue_ms: Option<f32>,

    sun_shadow_unnamed_ms: Option<f32>,

    sun_shadow_static_hit: Option<u32>,

    sun_shadow_world_ib_n: Option<u32>,

    sun_shadow_static_n: Option<u32>,

    sun_shadow_dynamic_n: Option<u32>,

    sun_shadow_wvp_intern_hit: Option<u32>,

    sun_shadow_wvp_intern_miss: Option<u32>,

    sun_shadow_wvp_intern_n: Option<u32>,

    sun_shadow_wvp_unique_base: Option<u32>,

    sun_shadow_wvp_unique_wvp: Option<u32>,

    sun_shadow_state_pipe_n: Option<u32>,

    sun_shadow_state_tess_n: Option<u32>,

    sun_shadow_state_bind_n: Option<u32>,

    sun_shadow_state_off_n: Option<u32>,

    sun_shadow_state_group_n: Option<u32>,

    sun_shadow_state_run_n: Option<u32>,

    sun_shadow_state_run_max: Option<u32>,

    sun_shadow_state_top10: Option<u32>,

    world_index_gaps: Option<u32>,

    world_run_indices_n: Option<u32>,

    world_material_runs: Option<u32>,

    world_material_runs_seq: Option<u32>,

    world_key_runs: Option<u32>,

    world_key_runs_seq: Option<u32>,

    world_mixed_breaks: Option<u32>,

    world_gathered: Option<u32>,

    world_ib_skip: Option<u32>,

    world_gpu_runs: Option<u32>,

    world_gpu_runs_seq: Option<u32>,

    world_sampler_runs_seq: Option<u32>,

    world_probe_runs_seq: Option<u32>,

    world_light_runs_seq: Option<u32>,

    smodel_reuse_n: Option<u32>,

    xmodel_reuse_n: Option<u32>,

    xmodel_material_runs: Option<u32>,

    smodel_index_gaps: Option<u32>,

    smodel_material_runs: Option<u32>,

    smodel_material_runs_seq: Option<u32>,

    smodel_material_run_max: Option<u32>,

    smodel_same_surface_n: Option<u32>,

    smodel_unique_surfaces: Option<u32>,

    smodel_hits: Option<u32>,

    smodel_lighting_runs: Option<u32>,
    smodel_lighting_run_max: Option<u32>,

    smodel_pretess_runs: Option<u32>,
    smodel_pretess_hits: Option<u32>,
    smodel_pretess_verts: Option<u32>,
    smodel_pretess_indices: Option<u32>,

    smodel_cached_lighting: Option<u32>,

    smodel_pretess_local: Option<u32>,

    smodel_pretess_length1: Option<u32>,

    smodel_pretess_skip: Option<u32>,

    submit_cause: Option<String>,

    submit_cause2: Option<String>,

    gpu_not_ready_n: Option<u32>,

    gpu_no_port_n: Option<u32>,

    pnr_smodel_mat: Option<String>,

    pnr_world_mat: Option<String>,

    pnr_smodel_ps: Option<String>,

    pnr_world_ps: Option<String>,

    pnr_smodel_key_n: Option<u32>,

    pnr_world_key_n: Option<u32>,

    pnr_port_n: Option<u32>,

    gpu_smodel_bind_mat: Option<String>,
}

fn reset_exact_colour_census(census: &mut ExactColourSubmitCensus) {
    census.submitted_keys.clear();
    census.bsp_submitted_surfaces = [0; 4];
    census.bsp_submit_refused_surfaces = [0; 4];
    census.bsp_drawn_surfaces = [0; 4];
    census.bsp_draw_refused_surfaces = [0; 4];
    census.submit_prepare_ms = None;
    census.colour_submit_ms = None;
    census.submit_encode_ms = None;
    census.submit_gather_ms = None;
    census.pass_end_ms = None;
    census.encoder_finish_ms = None;
    census.submit_arena_ms = None;
    census.submit_record_ms = None;
    census.pack_intern_hit_n = None;
    census.pack_intern_miss_n = None;
    census.pack_arena_share_n = None;
    census.gpu_exec_reuse_n = None;
    census.gpu_exec_unique_n = None;
    census.pack_overlay_n = None;
    census.pack_overlay_row_n = None;
    census.pack_overlay_pixel_share_n = None;
    census.pack_arena_vertex_n = None;
    census.pack_arena_pixel_n = None;
    census.pack_seed_n = None;
    census.pack_walk_n = None;
    census.tex_bind_hit_n = None;
    census.tex_bind_miss_n = None;
    census.markmesh_hits = None;
    census.markmesh_prepared = None;
    census.last_markmesh_refusal = None;
    census.last_markmesh_exec_skip = None;
    census.markmesh_missing_58 = None;
    census.last_mark_packed_custom = None;
    census.last_mark_packed_scene_light = None;
    census.last_mark_lmap_sampler = None;
    census.glassmesh_hits = None;
    census.glassmesh_prepared = None;
    census.last_glassmesh_exec_skip = None;
    census.last_glassmesh_refusal = None;
    census.last_glass_packed_probe = None;
    census.last_glass_probe_sampler = None;
    census.gpu_prepared = None;
    census.gpu_world_ready = None;
    census.gpu_smodel_ready = None;
    census.gpu_xmodel_ready = None;
    census.end_depth_restore_n = None;
    census.end_depth_range_type = None;
    census.submit_cause = None;
    census.submit_cause2 = None;
    census.gpu_not_ready_n = None;
    census.gpu_no_port_n = None;
    census.pnr_smodel_mat = None;
    census.pnr_world_mat = None;
    census.pnr_smodel_ps = None;
    census.pnr_world_ps = None;
    census.pnr_smodel_key_n = None;
    census.pnr_world_key_n = None;
    census.pnr_port_n = None;
    census.gpu_smodel_bind_mat = None;
    census.world_index_gaps = None;
    census.world_run_indices_n = None;
    census.world_material_runs = None;
    census.world_material_runs_seq = None;
    census.world_key_runs = None;
    census.world_key_runs_seq = None;
    census.world_mixed_breaks = None;
    census.world_gathered = None;
    census.world_ib_skip = None;
    census.world_gpu_runs = None;
    census.world_gpu_runs_seq = None;
    census.world_sampler_runs_seq = None;
    census.world_probe_runs_seq = None;
    census.world_light_runs_seq = None;
    census.smodel_reuse_n = None;
    census.xmodel_reuse_n = None;
    census.xmodel_material_runs = None;
    census.smodel_index_gaps = None;
    census.smodel_material_runs = None;
    census.smodel_material_runs_seq = None;
    census.smodel_material_run_max = None;
    census.smodel_same_surface_n = None;
    census.smodel_unique_surfaces = None;
    census.smodel_hits = None;
    census.smodel_lighting_runs = None;
    census.smodel_lighting_run_max = None;
    census.smodel_pretess_runs = None;
    census.smodel_pretess_hits = None;
    census.smodel_pretess_verts = None;
    census.smodel_pretess_indices = None;
    census.smodel_cached_lighting = None;
    census.smodel_pretess_local = None;
    census.smodel_pretess_length1 = None;
    census.smodel_pretess_skip = None;
    census.set_bind_group_n = None;
    census.set_state_n = None;
    census.multi_draw_n = None;
    census.multi_draw_commands_n = None;
    census.sun_shadow_gpu = None;
    census.sun_shadow_submit_ms = None;
    census.sun_shadow_gpu_miss = None;
    census.sun_shadow_gpu_cause = None;
    census.sun_shadow_gpu_causes = None;
    census.spot_shadow_gpu = None;
    census.spot_shadow_gpu_miss = None;
    census.spot_shadow_gpu_cause = None;
    census.spot_shadow_slot_n = None;
    census.sun_shadow_world_ib_n = None;
}

fn exec_tables(
    extracted: ExtractedColourRefs<'_>,
) -> Option<(&RuntimeMaterialCatalog, &PreparedMaterialTable)> {
    Some((
        extracted.world.catalog.as_deref()?,
        extracted.world.prepared.as_deref()?,
    ))
}

fn colour_census_clock(on: bool) -> Option<Instant> {
    on.then(Instant::now)
}

fn colour_census_ms(start: Option<Instant>) -> Option<f32> {
    start.map(|t| t.elapsed().as_secs_f32() * 1000.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RecordCensus {
    group0: u32,
    group1: u32,

    state: u32,

    multi_draws: u32,
    multi_draw_commands: u32,
}

impl RecordCensus {
    fn total(self) -> u32 {
        self.group0.saturating_add(self.group1)
    }

    fn add(&mut self, other: Self) {
        self.group0 = self.group0.saturating_add(other.group0);
        self.group1 = self.group1.saturating_add(other.group1);
        self.state = self.state.saturating_add(other.state);
        self.multi_draws = self.multi_draws.saturating_add(other.multi_draws);
        self.multi_draw_commands = self
            .multi_draw_commands
            .saturating_add(other.multi_draw_commands);
    }
}

#[derive(Default)]
struct ShadowExecScratch {
    executor: MaterialRunExecutor,
    code_sources: RuntimeCodeSources,
    pack_draws: Vec<PackDraw>,
}

#[derive(Resource, Default)]
struct ColourSubmitScratch {
    run_pack: RunPackCache,

    arena_pack: ArenaPack,

    executor: MaterialRunExecutor,

    world_exec_ready_keys: Vec<u64>,
    prepared: Vec<PreparedExactDraw>,
    submitted_keys: Vec<u64>,
    pending_viewmodel_prepared: Vec<PreparedExactDraw>,
    pending_viewmodel_keys: Vec<u64>,
    pack_draws: Vec<PackDraw>,

    pack_plan: Option<ColourPackPlan>,

    skinned_tess: smodel_skinned::SmodelSkinnedTess,

    prepared_scene_epoch: TableEpoch,
}

#[derive(Resource, Default)]
struct ShadowSubmitScratch {
    sun_exec: ShadowExecScratch,
    spot_exec: ShadowExecScratch,

    skinned_tess: smodel_skinned::SmodelSkinnedTess,

    sun_prepared: PreparedSunWork,
    spot_prepared: PreparedSpotWork,

    prepared_shadow_epoch: TableEpoch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorldPretessKey {
    world_generation: frame::WorldGeneration,
    geometry_generation: MaterialGenerationId,
    world_run_revision: u64,
    colour_world: u64,
    light_world: u64,
    emissive_world: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ColourPackKey {
    colour_digest: u64,
    light_digest: u64,
    emissive_digest: u64,
    pretess: WorldPretessKey,
    smodel_index_count: usize,
    xmodel_topology_revision: u64,
    xmodel_index_count: usize,
}

struct ColourRowPlan {
    draw_index: u32,
    world_surface_override: Option<u16>,

    technique: TechType,
    index_span: Option<(u32, u32)>,

    layout_epoch: u64,
}

struct ColourPackPlan {
    key: ColourPackKey,
    packed: PackedFrontendLists,
    work: render_backend::ColourDrawListWork,

    world_rows: Vec<Option<WorldPackedRowMeta>>,

    row_plan: Vec<ColourRowPlan>,
}

fn colour_pack_key(
    colour: &FrameProduct,
    light: &FrameProduct,
    emissive: &FrameProduct,
    pretess: WorldPretessKey,
    smodel_index_count: usize,
    xmodel_topology_revision: u64,
    xmodel_index_count: usize,
) -> ColourPackKey {
    ColourPackKey {
        colour_digest: colour.list_digest,
        light_digest: light.list_digest,
        emissive_digest: emissive.list_digest,
        pretess,
        smodel_index_count,
        xmodel_topology_revision,
        xmodel_index_count,
    }
}

fn mix_u64(id: &mut u64, word: u64) {
    *id ^= word;
    *id = id.wrapping_mul(0x0000_0100_0000_01b3);
}

fn mix_surface_samplers_gpu(id: &mut u64, samplers: super::SurfaceSamplerInputs) {
    let encode = |value: Option<u8>| value.map_or(0, |value| u64::from(value) + 1);
    mix_u64(id, encode(samplers.reflection_probe.map(|value| value.0)));
    mix_u64(id, encode(samplers.primary_lightmap.map(|value| value.0)));
    mix_u64(id, encode(samplers.secondary_lightmap.map(|value| value.0)));
}

fn expanded_world_id(product: &FrameProduct, run_surfs: &[u16]) -> u64 {
    let mut id = 0xcbf2_9ce4_8422_2325;
    for draw in &product.ordered_draws {
        let RetainedDrawKind::World {
            surf, run, run_off, ..
        } = draw.kind
        else {
            continue;
        };
        mix_u64(&mut id, u64::from(surf));
        mix_u64(&mut id, u64::from(run));
        mix_u64(&mut id, u64::from(run_off));
        mix_u64(&mut id, draw.key);
        mix_surface_samplers_gpu(&mut id, draw.surface_samplers);
        let members = run.max(1);
        for offset in 0..members {
            let member = if run <= 1 {
                surf
            } else {
                run_surfs
                    .get(run_off as usize + usize::from(offset))
                    .copied()
                    .unwrap_or(u16::MAX)
            };
            mix_u64(&mut id, u64::from(member));
        }
    }
    id
}

fn world_pretess_key(
    colour: &FrameProduct,
    light: &FrameProduct,
    emissive: &FrameProduct,
    run_surfs: &[u16],
    world_generation: frame::WorldGeneration,
    geometry_generation: MaterialGenerationId,
    world_run_revision: u64,
) -> WorldPretessKey {
    WorldPretessKey {
        world_generation,
        geometry_generation,
        world_run_revision,
        colour_world: expanded_world_id(colour, run_surfs),
        light_world: expanded_world_id(light, run_surfs),
        emissive_world: expanded_world_id(emissive, run_surfs),
    }
}

#[derive(Resource, Default)]
struct CameraPrepareState {
    active: bool,
    samples: u32,
    needs_floatz: bool,
    needs_resolved_scene: bool,
    world_ib_skip: bool,
    ready_draws: u32,
    refused_draws: u32,
    pipeline_not_ready: u32,
    last_refusal: Option<GpuSubmitRefusal>,
    submit_refusals: BTreeMap<(&'static str, &'static str), u32>,
    exec_refused: u32,
    exec_refusals: BTreeMap<(&'static str, &'static str), u32>,
    unsupported_state: UnsupportedStateCensus,
    bsp_submit_refused_surfaces: [u32; 4],
    pnr_smodel_mats: BTreeMap<String, u32>,
    pnr_world_mats: BTreeMap<String, u32>,
    pnr_smodel_ps: BTreeMap<String, u32>,
    pnr_world_ps: BTreeMap<String, u32>,
    pnr_smodel_keys: HashSet<u64>,
    pnr_world_keys: HashSet<u64>,
    pnr_ports: HashSet<PortId>,
    bind_smodel_mats: BTreeMap<String, u32>,
    last_markmesh_refusal: Option<GpuSubmitRefusal>,
    last_markmesh_exec_skip: Option<&'static str>,
    markmesh_missing_58: u32,
    last_glassmesh_exec_skip: Option<&'static str>,
    last_glassmesh_refusal: Option<GpuSubmitRefusal>,
    last_mark_packed_custom: Option<u8>,
    last_mark_packed_scene_light: Option<u8>,
    last_mark_lmap_sampler: Option<u32>,
    last_glass_packed_probe: Option<u8>,
    last_glass_probe_sampler: Option<u32>,
    markmesh_hits: usize,
    glassmesh_hits: usize,
    prepared_hits: u32,
    prepare_cost: PrepareCost,
    colour_run_census: render_backend::MaterialRunCensus,
    focused_object_id: Option<u16>,
    viewmodel_held: usize,
}

impl ColourSubmitScratch {
    fn clear(&mut self) {
        self.world_exec_ready_keys.clear();
        self.prepared.clear();
        self.submitted_keys.clear();
        self.pending_viewmodel_prepared.clear();
        self.pending_viewmodel_keys.clear();
    }
}

pub fn bind_group_layout_from_entries(
    label: &'static str,
    entries: &[WgpuBindLayoutEntry],
    dynamic_uniforms: bool,
) -> BindGroupLayoutDescriptor {
    let mut built = Vec::with_capacity(entries.len());
    for entry in entries {
        let visibility = match entry.visibility {
            WgpuShaderVisibility::Vertex => ShaderStages::VERTEX,
            WgpuShaderVisibility::Fragment => ShaderStages::FRAGMENT,
            WgpuShaderVisibility::VertexFragment => ShaderStages::VERTEX_FRAGMENT,
        };
        let builder = match entry.kind {
            WgpuBindingKind::UniformBuffer { min_size } => {
                uniform_buffer_sized(dynamic_uniforms, NonZeroU64::new(min_size))
                    .visibility(visibility)
            }
            WgpuBindingKind::ReadOnlyStorageBuffer => {
                storage_buffer_read_only::<[u32; 4]>(false).visibility(visibility)
            }
            WgpuBindingKind::TextureArray { dimension, count } => {
                let sample = TextureSampleType::Float { filterable: true };
                match dimension {
                    super::SamplerTextureDimension::D2 => texture_2d(sample),
                    super::SamplerTextureDimension::Cube => texture_cube(sample),
                    super::SamplerTextureDimension::D3 => texture_3d(sample),
                }
                .visibility(visibility)
                .count(texture_table::array_count(count))
            }
            WgpuBindingKind::SamplerArray { count } => sampler(SamplerBindingType::Filtering)
                .visibility(visibility)
                .count(texture_table::array_count(count)),
        };
        built.push(builder.build(u32::from(entry.binding), visibility));
    }
    BindGroupLayoutDescriptor::new(label, &built)
}

pub fn vertex_layouts_from_contract(
    layout: &super::gpu_contract::WgpuPassLayout,
) -> Vec<VertexBufferLayout> {
    layout
        .vertex_buffers
        .iter()
        .map(|buffer| VertexBufferLayout {
            array_stride: buffer.array_stride,
            step_mode: VertexStepMode::Vertex,
            attributes: buffer
                .attributes
                .iter()
                .map(|attribute| VertexAttribute {
                    format: match attribute.format {
                        WgpuVertexFormat::Float32x2 => VertexFormat::Float32x2,
                        WgpuVertexFormat::Float32x3 => VertexFormat::Float32x3,
                        WgpuVertexFormat::Float32x4 => VertexFormat::Float32x4,
                        WgpuVertexFormat::Unorm8x4 => VertexFormat::Unorm8x4,
                        WgpuVertexFormat::Uint8x4 => VertexFormat::Uint8x4,
                    },
                    offset: attribute.offset,
                    shader_location: attribute.location,
                })
                .collect(),
        })
        .collect()
}

pub fn colour_ports_static(
    extracted_generation: MaterialGenerationId,
    extracted_port_len: usize,
    cpu_generation: MaterialGenerationId,
    cpu_port_len: usize,
) -> bool {
    extracted_generation == cpu_generation
        && extracted_port_len == cpu_port_len
        && extracted_port_len > 0
}

pub fn colour_world_smodel_static(
    (gpu_world_generation, gpu_world_products): (frame::WorldGeneration, frame::WorldProducts),
    gpu_world_verts: usize,
    gpu_world_indices: usize,
    gpu_world_layer: usize,
    gpu_smodel_verts: usize,
    gpu_smodel_indices: usize,
    (cpu_world_generation, cpu_world_products): (frame::WorldGeneration, frame::WorldProducts),
    cpu_world_verts: usize,
    cpu_world_indices: usize,
    cpu_world_layer: usize,
    cpu_smodel_verts: usize,
    cpu_smodel_indices: usize,
) -> bool {
    (gpu_world_generation == cpu_world_generation || gpu_world_products.same_as(cpu_world_products))
        && gpu_world_verts == cpu_world_verts
        && gpu_world_indices == cpu_world_indices
        && gpu_world_layer == cpu_world_layer
        && gpu_smodel_verts == cpu_smodel_verts
        && gpu_smodel_indices == cpu_smodel_indices
}

fn is_viewmodel_colour_draw(kind: &RetainedDrawKind, key: u64) -> bool {
    depth_range_type_for_draw(kind, key) == GFX_DEPTH_RANGE_VIEWMODEL
}

fn viewmodel_colour_submits_when_pipelines_ready(any_viewmodel_pipeline_not_ready: bool) -> bool {
    !any_viewmodel_pipeline_not_ready
}

fn sun_shadow_view_missing(view_ready: bool, binds_sun_shadow: bool) -> bool {
    binds_sun_shadow && !view_ready
}

fn sun_shadow_content_missing(sun_recorded: bool, binds_sun_shadow: bool) -> bool {
    binds_sun_shadow && !sun_recorded
}

fn retain_camera_draws_with_sun_content(
    prepared: &mut Vec<PreparedExactDraw>,
    sun_recorded: bool,
    spot_recorded: bool,
) -> u32 {
    let mut held = 0u32;
    prepared.retain(|draw| {
        if sun_shadow_content_missing(sun_recorded, draw.binds_sun_shadow)
            || (draw.binds_spot_shadow && !spot_recorded)
        {
            held = held.saturating_add(1);
            false
        } else {
            true
        }
    });
    held
}

fn publish_this_frame_sun_shadow_view(
    products: &ExtractedRenderFrameProducts,
    uploaded: &mut RuntimeUploadedImageRegistry,
    shadowmap: &mut ShadowmapSunGpu,
    device: &RenderDevice,
) -> bool {
    let sun = products.0.product(FrameProductKind::SunShadow);
    if sun.ordered_draws.is_empty() {
        uploaded.publish_frame_target(|registry| &mut registry.sun_shadow, None);
        return false;
    }

    let (color_view, _resized) = ensure_shadowmap_sun_target(shadowmap, device);
    uploaded.publish_frame_target(|registry| &mut registry.sun_shadow, color_view);
    uploaded.sun_shadow.is_some()
}

fn spot_rt_for_light(products: &ExtractedRenderFrameProducts, light_index: u8) -> Option<u8> {
    products
        .0
        .product(FrameProductKind::SpotShadow)
        .spot_slots
        .iter()
        .find(|slot| slot.emitted.light_index == light_index)
        .map(|slot| slot.emitted.plan.render_target_id)
}

fn spot_shadow_view_missing(
    uploaded: &RuntimeUploadedImageRegistry,
    binds: bool,
    select: Option<u8>,
) -> bool {
    if !binds {
        return false;
    }
    let Some(rt) = select else {
        return true;
    };
    match rt {
        lighting_iw4::GFX_SPOT_SHADOW_RT_LARGE => uploaded.spot_shadow_rt10.is_none(),
        lighting_iw4::GFX_SPOT_SHADOW_RT_SMALL => uploaded.spot_shadow_rt11.is_none(),
        _ => true,
    }
}

fn publish_this_frame_spot_shadow_views(
    products: &ExtractedRenderFrameProducts,
    uploaded: &mut RuntimeUploadedImageRegistry,
    shadowmap: &mut ShadowmapSpotGpu,
    device: &RenderDevice,
) -> bool {
    let spot = products.0.product(FrameProductKind::SpotShadow);
    if spot.spot_slots.is_empty() {
        uploaded.publish_frame_target(|registry| &mut registry.spot_shadow_rt10, None);
        uploaded.publish_frame_target(|registry| &mut registry.spot_shadow_rt11, None);
        return false;
    }
    ensure_shadowmap_spot_targets(shadowmap, device);

    let large = shadowmap
        .color_view(lighting_iw4::GFX_SPOT_SHADOW_RT_LARGE)
        .cloned();
    let small = shadowmap
        .color_view(lighting_iw4::GFX_SPOT_SHADOW_RT_SMALL)
        .cloned();
    uploaded.publish_frame_target(|registry| &mut registry.spot_shadow_rt10, large);
    uploaded.publish_frame_target(|registry| &mut registry.spot_shadow_rt11, small);
    uploaded.spot_shadow_rt10.is_some() || uploaded.spot_shadow_rt11.is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum XModelUpload {
    Resident,

    Rewrite { vertices: bool, indices: bool },

    Grow,
}

struct XModelUploadQuery {
    uploaded_vertices: u64,
    uploaded_topology: u64,
    resident_verts: usize,
    resident_indices: usize,

    vertex_fits: bool,
    index_fits: bool,
    cpu_vertices_revision: u64,
    cpu_topology_revision: u64,
    cpu_verts: usize,
    cpu_indices: usize,
}

fn colour_xmodel_upload(q: XModelUploadQuery) -> XModelUpload {
    if !q.vertex_fits || !q.index_fits {
        return XModelUpload::Grow;
    }
    let vertices =
        q.uploaded_vertices != q.cpu_vertices_revision || q.resident_verts != q.cpu_verts;
    let indices =
        q.uploaded_topology != q.cpu_topology_revision || q.resident_indices != q.cpu_indices;
    if !vertices && !indices {
        return XModelUpload::Resident;
    }
    XModelUpload::Rewrite { vertices, indices }
}

fn colour_code_mesh_upload_kind(
    gpu_revision: u64,
    gpu_verts: usize,
    gpu_indices: usize,
    gpu_has_ring: bool,
    cpu_revision: u64,
    cpu_verts: usize,
    cpu_indices: usize,
) -> i32 {
    let over =
        cpu_verts > CODE_MESH_VERT_CAP as usize || cpu_indices > CODE_MESH_INDEX_CAP as usize;
    if over {
        return 0;
    }
    if cpu_verts == 0 || cpu_indices == 0 {
        return 0;
    }
    if gpu_has_ring
        && gpu_revision == cpu_revision
        && gpu_verts == cpu_verts
        && gpu_indices == cpu_indices
    {
        0
    } else if gpu_has_ring {
        1
    } else {
        2
    }
}

fn drawsurf_object_id(key: u64) -> u16 {
    dpvs_iw4::GfxDrawSurf::from_packed(key).object_id()
}

#[derive(Resource, Default)]
pub struct FocusedOwnerSubmitState {
    emitted_frame_id: u64,
}

pub fn emit_focused_owner_submit(
    products: &ExtractedRenderFrameProducts,
    state: &mut FocusedOwnerSubmitState,
    material_context: Option<(&[String], &super::gpu_resources::RuntimeImageHandles)>,
    tables: Option<(&RuntimeMaterialCatalog, &PreparedMaterialTable)>,
    terminal: &'static str,
    prepared_surfaces: u32,
    prepared_passes: u32,
    drawn_passes: u32,
) {
    let Some(focus) = products.0.focus() else {
        return;
    };
    if state.emitted_frame_id == products.0.frame_id {
        return;
    }
    let mut product_surfaces = 0u32;
    let mut execution_ready_surfaces = 0u32;
    let mut materials = BTreeSet::new();
    let mut material_textures = BTreeSet::new();
    if let Some(object_id) = focus.object_id {
        for product in [
            products.0.product(FrameProductKind::Colour),
            products.0.product(FrameProductKind::Light),
            products.0.product(FrameProductKind::Emissive),
        ] {
            for (index, item) in product.ordered_draws.iter().enumerate() {
                if drawsurf_object_id(item.key) != object_id {
                    continue;
                }
                product_surfaces = product_surfaces.saturating_add(1);
                let resolved = tables.and_then(|(catalog, prepared)| {
                    let tech = product.draw_tech.get(index).copied()?;
                    prepared_draw_technique(
                        catalog,
                        prepared,
                        render_material::MaterialDrawKey::new(item.key, item.material_rank)
                            .with_material_id(item.material_id),
                        tech,
                    )
                    .ok()
                });
                execution_ready_surfaces =
                    execution_ready_surfaces.saturating_add(u32::from(resolved.is_some()));
                let ordinal = world_material_sorted(item.key);
                let material = material_context
                    .and_then(|(names, _)| names.get(usize::from(ordinal)))
                    .map_or_else(|| ordinal.to_string(), |name| format!("{ordinal}:{name}"));
                materials.insert(material);
                if let Some((_, prepared_tech)) = resolved {
                    for (pass_index, pass) in prepared_tech.passes.iter().enumerate() {
                        let Some(local) = pass.local_samplers.as_ref() else {
                            continue;
                        };
                        for lane in &local.lanes {
                            let image_index = lane.texture.image.0 as usize;
                            let image = material_context.map_or_else(String::new, |(_, images)| {
                                let image_name = images
                                    .material_names
                                    .get(image_index)
                                    .map(String::as_str)
                                    .unwrap_or("<out-of-range>");
                                let retained = images
                                    .material_images
                                    .get(image_index)
                                    .is_some_and(Option::is_some);
                                format!(",image_name={image_name},retained={}", u8::from(retained))
                            });
                            material_textures.insert(format!(
                                "material={ordinal},pass={pass_index},register={},name_hash=0x{:08x},image={},semantic={},sampler=0x{:02x}{image}",
                                lane.register,
                                lane.name_hash,
                                lane.texture.image.0,
                                lane.texture.semantic,
                                lane.texture.sampler_state,
                            ));
                        }
                    }
                }
            }
        }
    }
    let materials =
        (!materials.is_empty()).then(|| materials.into_iter().collect::<Vec<_>>().join(";"));
    let material_textures = (!material_textures.is_empty())
        .then(|| material_textures.into_iter().collect::<Vec<_>>().join(";"));
    let outcome = if focus.outcome == "planned_empty" {
        "planned_empty"
    } else if focus.outcome != "planned" {
        "plan_refused"
    } else if terminal != "complete" {
        terminal
    } else if product_surfaces == 0 {
        "product_missing"
    } else if execution_ready_surfaces == 0 {
        "material_refused"
    } else if execution_ready_surfaces < product_surfaces {
        "material_partial"
    } else if prepared_surfaces == 0 {
        "gpu_prepare_refused"
    } else if prepared_surfaces < execution_ready_surfaces {
        "gpu_prepare_partial"
    } else if drawn_passes == 0 {
        "draw_refused"
    } else if drawn_passes < prepared_passes {
        "draw_partial"
    } else {
        "drawn"
    };
    perf::render_owner_submit(
        products.0.frame_id,
        "script_model",
        focus.owner_id,
        focus.object_id,
        outcome,
        materials.as_deref(),
        material_textures.as_deref(),
        product_surfaces,
        execution_ready_surfaces,
        prepared_surfaces,
        prepared_passes,
        drawn_passes,
    );
    state.emitted_frame_id = products.0.frame_id;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct UnsupportedStateCensus {
    unknown_blend_factor: u32,
    unknown_blend_operation: u32,
    stencil: u32,
}

impl UnsupportedStateCensus {
    fn note(&mut self, fields: super::state::UnsupportedStateFields) {
        self.unknown_blend_factor = self
            .unknown_blend_factor
            .saturating_add(u32::from(fields.unknown_blend_factor));
        self.unknown_blend_operation = self
            .unknown_blend_operation
            .saturating_add(u32::from(fields.unknown_blend_operation));
        self.stencil = self.stencil.saturating_add(u32::from(fields.stencil));
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AuthoredStateCensus {
    non_add_blend: u32,
    independent_alpha_blend: u32,
    partial_colour_write: u32,
    line_fill: u32,
    stencil: u32,
}

impl AuthoredStateCensus {
    fn note(&mut self, fields: super::state::AuthoredStateFields) {
        self.non_add_blend = self
            .non_add_blend
            .saturating_add(u32::from(fields.non_add_blend));
        self.independent_alpha_blend = self
            .independent_alpha_blend
            .saturating_add(u32::from(fields.independent_alpha_blend));
        self.partial_colour_write = self
            .partial_colour_write
            .saturating_add(u32::from(fields.partial_colour_write));
        self.line_fill = self.line_fill.saturating_add(u32::from(fields.line_fill));
        self.stencil = self.stencil.saturating_add(u32::from(fields.stencil));
    }
}

fn as_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn world_pretess_dest_ib(
    device: &RenderDevice,
    queue: &RenderQueue,
    reuse: Option<Buffer>,
    indices: &[u32],
) -> Option<Buffer> {
    if indices.is_empty() {
        return None;
    }
    let bytes: &[u8] = bytemuck::cast_slice(indices);
    let need = padded_upload_len(bytes.len()) as u64;
    let buf = match reuse {
        Some(existing) if existing.size() >= need => existing,
        _ => device.create_buffer(&BufferDescriptor {
            label: Some("iw4_world_pretess_dest_ib"),
            size: need,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }),
    };
    write_buffer_padded(queue, &buf, bytes);
    Some(buf)
}

fn smodel_pretess_submit_refusal(
    extracted: ExtractedColourRefs<'_>,
    placement: u32,
    cache_index: u16,
    range: SmodelPretessRange,
) -> Result<(ExactTessBind, u32, u32, u8), GpuSubmitRefusal> {
    if cache_index == 0 {
        return Err(GpuSubmitRefusal::SmodelCacheIndexEmpty { placement });
    }
    if !extracted.world.smc_index_baked.contains(&cache_index) {
        return Err(GpuSubmitRefusal::SmodelCacheIndicesMissing { cache_index });
    }
    let end = range
        .start
        .checked_add(range.count)
        .and_then(|end| usize::try_from(end).ok());
    if range.count == 0
        || range.count % 3 != 0
        || end.is_none_or(|end| end > extracted.world.smodel_pretess_indices.len())
    {
        return Err(GpuSubmitRefusal::SmodelCacheIndicesMissing { cache_index });
    }
    Ok((
        ExactTessBind::SmodelCached,
        range.start,
        range.count,
        asset_iw4::vertex_decl::STATICMODELCACHE_VERTEX_TYPE,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExactTessBind {
    World,
    Smodel,
    SmodelCached,
    SmodelSkinned,
    XModel,
    CodeMesh,
    ParticleCloud,
    MarkMesh,
    Glass,
}

#[derive(Clone)]
struct PreparedExactDraw {
    pipeline: ExactPipelineSlot,
    port: PortId,

    constants: Option<Arc<PassConstantBuffers>>,

    constant_base: Option<u32>,

    texture_slots: Arc<[u32]>,
    start: u32,
    count: u32,
    tess: ExactTessBind,

    after_scene_resolve: bool,

    owner_object_id: Option<u16>,

    depth_min: f32,
    depth_max: f32,

    ring_epoch: u32,

    smc_stream_off: u64,

    state: super::state::GfxPassState,

    bsp_kind: Option<BspCameraLane>,
    bsp_run_first: u16,
    bsp_first_surf: u16,
    bsp_surf_count: u16,

    bsp_counted: bool,

    binds_sun_shadow: bool,

    binds_spot_shadow: bool,

    indirect_arg: Option<u32>,
}

fn bsp_draw_source(kind: &RetainedDrawKind) -> (Option<BspCameraLane>, u16, u16, u16) {
    match *kind {
        RetainedDrawKind::World {
            surf,
            run,
            bsp_kind: Some(kind),
            bsp_run_first: Some(run_first),
            ..
        } => (Some(kind), run_first, surf, run.max(1)),
        RetainedDrawKind::World {
            bsp_kind: Some(_),
            bsp_run_first: None,
            ..
        } => panic!("BSP world draw is missing its producer run identity"),
        _ => (None, 0, 0, 0),
    }
}

const fn bsp_kind_index(kind: BspCameraLane) -> usize {
    match kind {
        BspCameraLane::LitOpaque => 0,
        BspCameraLane::LitTrans => 1,
        BspCameraLane::Emissive => 2,
        BspCameraLane::Decal => 3,
    }
}

fn submit_refusal_family(kind: &RetainedDrawKind, viewmodel: bool) -> &'static str {
    match kind {
        RetainedDrawKind::World { .. } => "world",
        RetainedDrawKind::Smodel { .. } => "smodel",
        RetainedDrawKind::XModel { .. } if viewmodel => "xmodel/fpv",
        RetainedDrawKind::XModel { .. } => "xmodel",
        RetainedDrawKind::CodeMesh { .. } => "codemesh",
        RetainedDrawKind::ParticleCloud { .. } => "particlecloud",
        RetainedDrawKind::MarkMesh { .. } => "markmesh",
        RetainedDrawKind::Glass { .. } => "glassmesh",
    }
}

fn submit_refusal_class(cause: &GpuSubmitRefusal) -> &'static str {
    match cause {
        GpuSubmitRefusal::NoExactPort { .. } => "NoExactPort",
        GpuSubmitRefusal::ConstantPack(_) => "ConstantPack",
        GpuSubmitRefusal::TextureBind(_) => "TextureBind",
        GpuSubmitRefusal::TextureTable(_) => "TextureTable",
        GpuSubmitRefusal::ProductDependencyNotReady { .. } => "ProductDependencyNotReady",
        GpuSubmitRefusal::UnsupportedTessKind => "UnsupportedTessKind",
        GpuSubmitRefusal::MultipleCameraViews { .. } => "MultipleCameraViews",
        GpuSubmitRefusal::WorldVertex(_) => "WorldVertex",
        GpuSubmitRefusal::PackedVertex(_) => "PackedVertex",
        GpuSubmitRefusal::MissingSurfaceRange { .. } => "MissingSurfaceRange",
        GpuSubmitRefusal::MissingSmodelRange { .. } => "MissingSmodelRange",
        GpuSubmitRefusal::MissingXModelRange { .. } => "MissingXModelRange",
        GpuSubmitRefusal::MissingCodeMeshRange { .. } => "MissingCodeMeshRange",
        GpuSubmitRefusal::MissingParticleCloudRange { .. } => "MissingParticleCloudRange",
        GpuSubmitRefusal::MissingMarkMeshRange { .. } => "MissingMarkMeshRange",
        GpuSubmitRefusal::MissingGlassMeshRange { .. } => "MissingGlassMeshRange",
        GpuSubmitRefusal::EmptyIndexRange { .. } => "EmptyIndexRange",
        GpuSubmitRefusal::EmptySmodelIndexRange { .. } => "EmptySmodelIndexRange",
        GpuSubmitRefusal::EmptyXModelIndexRange { .. } => "EmptyXModelIndexRange",
        GpuSubmitRefusal::EmptyCodeMeshIndexRange { .. } => "EmptyCodeMeshIndexRange",
        GpuSubmitRefusal::EmptyParticleCloudIndexRange { .. } => "EmptyParticleCloudIndexRange",
        GpuSubmitRefusal::EmptyMarkMeshIndexRange { .. } => "EmptyMarkMeshIndexRange",
        GpuSubmitRefusal::EmptyGlassMeshIndexRange { .. } => "EmptyGlassMeshIndexRange",
        GpuSubmitRefusal::PipelineNotReady => "PipelineNotReady",
        GpuSubmitRefusal::SmodelCacheIndexEmpty { .. } => "SmodelCacheIndexEmpty",
        GpuSubmitRefusal::SmodelCacheIndicesMissing { .. } => "SmodelCacheIndicesMissing",
        GpuSubmitRefusal::SmodelXSurfacePathUnread { .. } => "SmodelXSurfacePathUnread",
        GpuSubmitRefusal::SmodelCachedWithoutDestRange { .. } => "SmodelCachedWithoutDestRange",
        GpuSubmitRefusal::SmodelSkinnedDestMissing { .. } => "SmodelSkinnedDestMissing",
        GpuSubmitRefusal::ConstantArenaMissing => "ConstantArenaMissing",
        GpuSubmitRefusal::WorldPretessSpanBeyondLimit { .. } => "WorldPretessSpanBeyondLimit",
        GpuSubmitRefusal::WorldPretessEpochMismatch { .. } => "WorldPretessEpochMismatch",
    }
}

fn exec_refusal_row(
    kind: &RetainedDrawKind,
    key: u64,
    cause: &MaterialRefusal,
) -> (&'static str, &'static str) {
    let viewmodel = is_viewmodel_colour_draw(kind, key);
    (
        submit_refusal_family(kind, viewmodel),
        material_refusal_class(cause),
    )
}

struct DrawRefusalCensus {
    on: bool,
    submit: BTreeMap<(&'static str, &'static str), u32>,
    exec: BTreeMap<(&'static str, &'static str), u32>,
}

impl DrawRefusalCensus {
    fn new(on: bool) -> Self {
        Self {
            on,
            submit: BTreeMap::new(),
            exec: BTreeMap::new(),
        }
    }

    fn taking(
        on: bool,
        submit: BTreeMap<(&'static str, &'static str), u32>,
        exec: BTreeMap<(&'static str, &'static str), u32>,
    ) -> Self {
        Self { on, submit, exec }
    }

    fn note_exec(&mut self, kind: &RetainedDrawKind, key: u64, cause: &MaterialRefusal) {
        if !self.on {
            return;
        }
        *self
            .exec
            .entry(exec_refusal_row(kind, key, cause))
            .or_default() += 1;
    }

    fn note_submit(&mut self, kind: &RetainedDrawKind, viewmodel: bool, cause: &GpuSubmitRefusal) {
        if !self.on {
            return;
        }
        *self
            .submit
            .entry((
                submit_refusal_family(kind, viewmodel),
                submit_refusal_class(cause),
            ))
            .or_default() += 1;
    }

    fn note_submit_class(&mut self, family: &'static str, class: &'static str, n: u32) {
        if !self.on || n == 0 {
            return;
        }
        *self.submit.entry((family, class)).or_default() += n;
    }
}

fn material_refusal_class(cause: &MaterialRefusal) -> &'static str {
    match cause {
        MaterialRefusal::SortedMaterialTableMissing { .. } => "SortedMaterialTableMissing",
        MaterialRefusal::SortedMaterialOrdinalOutOfRange { .. } => {
            "SortedMaterialOrdinalOutOfRange"
        }
        MaterialRefusal::SortedMaterialBuildFailed { .. } => "SortedMaterialBuildFailed",
        MaterialRefusal::StaleMaterialGeneration { .. } => "StaleMaterialGeneration",
        MaterialRefusal::MaterialOutOfRange { .. } => "MaterialOutOfRange",
        MaterialRefusal::LocalTechniqueSetOutOfRange { .. } => "LocalTechniqueSetOutOfRange",
        MaterialRefusal::RemappedTechniqueSetOutOfRange { .. } => "RemappedTechniqueSetOutOfRange",
        MaterialRefusal::RemapMissing { .. } => "RemapMissing",
        MaterialRefusal::RemapCycle { .. } => "RemapCycle",
        MaterialRefusal::TechniqueAbsent { .. } => "TechniqueAbsent",
        MaterialRefusal::EmptyTechnique { .. } => "EmptyTechnique",
        MaterialRefusal::StateEntriesMissing => "StateEntriesMissing",
        MaterialRefusal::StateEntryOutOfRange { .. } => "StateEntryOutOfRange",
        MaterialRefusal::StateRowOutOfRange { .. } => "StateRowOutOfRange",
        MaterialRefusal::UnsupportedState { .. } => "UnsupportedState",
        MaterialRefusal::PassIndexOverflow { .. } => "PassIndexOverflow",
        MaterialRefusal::ShaderProgramMissing { .. } => "ShaderProgramMissing",
        MaterialRefusal::UnsupportedShaderPair { .. } => "UnsupportedShaderPair",
        MaterialRefusal::MissingTexture { .. } => "MissingTexture",
        MaterialRefusal::MissingMaterialConstant { .. } => "MissingMaterialConstant",
        MaterialRefusal::MissingCodeConstant { .. } => "MissingCodeConstant",
        MaterialRefusal::CodeConstantRowsOutOfRange { .. } => "CodeConstantRowsOutOfRange",
        MaterialRefusal::MissingCodeTexture { .. } => "MissingCodeTexture",
        MaterialRefusal::MissingLiteralConstant { .. } => "MissingLiteralConstant",
        MaterialRefusal::UnknownArgumentType { .. } => "UnknownArgumentType",
        MaterialRefusal::PassArgCountMismatch { .. } => "PassArgCountMismatch",
        MaterialRefusal::TechniqueSetNamespaceMismatch { .. } => "TechniqueSetNamespaceMismatch",
    }
}

pub fn dump_sorted_material_names(catalog: &RuntimeMaterialCatalog) -> Vec<String> {
    match &catalog.sorted_materials {
        RuntimeSortedMaterialTable::Ready {
            asset_ids_by_ordinal,
            ..
        } => asset_ids_by_ordinal
            .iter()
            .map(|id| {
                catalog
                    .materials
                    .get(usize::from(id.0))
                    .map(|material| material.name.clone())
                    .unwrap_or_else(|| format!("id{}", id.0))
            })
            .collect(),
        RuntimeSortedMaterialTable::Missing { .. } | RuntimeSortedMaterialTable::BuildFailed(_) => {
            Vec::new()
        }
    }
}

pub fn dump_shader_program_names(catalog: &RuntimeMaterialCatalog) -> Vec<Option<String>> {
    catalog
        .shader_programs
        .iter()
        .map(|slot| slot.as_ref().map(|program| program.name.clone()))
        .collect()
}

fn rank_count_map(map: &BTreeMap<String, u32>, take: usize) -> Option<String> {
    if map.is_empty() {
        return None;
    }
    let mut ranked: Vec<(&String, &u32)> = map.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    Some(
        ranked
            .into_iter()
            .take(take)
            .map(|(name, n)| format!("{name}:{n}"))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn rank_pair_map(map: &BTreeMap<(&'static str, &'static str), u32>, take: usize) -> Option<String> {
    if map.is_empty() {
        return None;
    }
    let mut ranked: Vec<_> = map.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    Some(
        ranked
            .into_iter()
            .take(take)
            .map(|((family, cause), n)| format!("{family}:{cause}:{n}"))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn record_pipeline_not_ready(
    kind: &RetainedDrawKind,
    viewmodel: bool,
    key: u64,
    execution: &MaterialExecution,
    extracted: ExtractedColourRefs<'_>,
    smodel_mats: &mut BTreeMap<String, u32>,
    world_mats: &mut BTreeMap<String, u32>,
    smodel_ps: &mut BTreeMap<String, u32>,
    world_ps: &mut BTreeMap<String, u32>,
    smodel_keys: &mut HashSet<u64>,
    world_keys: &mut HashSet<u64>,
    ports: &mut HashSet<PortId>,
) {
    let family = submit_refusal_family(kind, viewmodel);
    let ordinal = world_material_sorted(key);
    let name = extracted
        .world
        .sorted_material_names
        .get(usize::from(ordinal))
        .cloned()
        .unwrap_or_else(|| format!("ord{ordinal}"));
    let ps = execution
        .pass(0)
        .map(|pass| {
            extracted
                .world
                .shader_program_names
                .get(pass.shader_pair.pixel.asset_slot as usize)
                .and_then(|slot| slot.clone())
                .unwrap_or_else(|| format!("{:016x}", pass.port.pixel_program_hash))
        })
        .unwrap_or_else(|| "<no pass>".to_owned());
    for pass in execution.iter_passes() {
        ports.insert(pass.port);
    }
    match family {
        "smodel" => {
            *smodel_mats.entry(name).or_default() += 1;
            *smodel_ps.entry(ps).or_default() += 1;
            smodel_keys.insert(key);
        }
        "world" => {
            *world_mats.entry(name).or_default() += 1;
            *world_ps.entry(ps).or_default() += 1;
            world_keys.insert(key);
        }
        _ => {}
    }
}

fn execution_binds_code_texture(execution: &MaterialExecution, index: u32) -> bool {
    execution
        .iter_passes()
        .any(|pass| pass.code_samplers.iter().any(|lane| lane.index == index))
}

fn identity_placement_kind(kind: &RetainedDrawKind) -> bool {
    match *kind {
        RetainedDrawKind::World { .. }
        | RetainedDrawKind::CodeMesh { .. }
        | RetainedDrawKind::MarkMesh { .. }
        | RetainedDrawKind::Glass { .. } => true,
        RetainedDrawKind::Smodel {
            world_from_local, ..
        }
        | RetainedDrawKind::XModel {
            world_from_local, ..
        } => world_from_local == Mat4::IDENTITY,
        _ => false,
    }
}

#[derive(Default)]
struct RunPackCache {
    serial: u64,
    passes: Vec<Option<Arc<PassConstantBuffers>>>,

    pipelines: Vec<Option<(usize, ExactPipelineSlot)>>,
    intern: HashMap<PackedBankKey, (Arc<PassConstantBuffers>, u32)>,
    intern_stamp: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PackedBankKey {
    port: PortId,
    local: usize,
    code: u64,
}

fn packed_bank_key(
    port: PortId,
    local: Option<&Arc<render_material::PackedLocalBanks>>,
    code: u64,
) -> PackedBankKey {
    PackedBankKey {
        port,
        local: local.map(|banks| Arc::as_ptr(banks) as usize).unwrap_or(0),
        code,
    }
}

fn intern_packed_bank<E>(
    intern: &mut HashMap<PackedBankKey, (Arc<PassConstantBuffers>, u32)>,
    stamp: u32,
    key: PackedBankKey,
    miss: impl FnOnce() -> Result<Arc<PassConstantBuffers>, E>,
) -> Result<(Arc<PassConstantBuffers>, bool), E> {
    if let Some((existing, seen)) = intern.get_mut(&key) {
        *seen = stamp;
        return Ok((Arc::clone(existing), true));
    }
    let packed = miss()?;
    intern.insert(key, (Arc::clone(&packed), stamp));
    Ok((packed, false))
}

impl RunPackCache {
    fn begin(&mut self, serial: u64, pass_len: usize) {
        if self.serial != serial {
            self.serial = serial;
            self.passes.clear();
            self.pipelines.clear();
        }
        if self.passes.len() < pass_len {
            self.passes.resize(pass_len, None);
            self.pipelines.resize(pass_len, None);
        }
    }

    fn begin_pack_intern_frame(&mut self) {
        self.intern_stamp = self.intern_stamp.wrapping_add(1);
    }

    fn sweep_pack_intern(&mut self) {
        self.intern
            .retain(|_, (_, seen)| *seen == self.intern_stamp);
    }

    fn pack(
        &mut self,
        pass_index: usize,
        port: &super::AdmittedExactPort,
        executable: ExecutablePassView<'_>,
        prepare_cost: &mut PrepareCost,
    ) -> Result<Arc<PassConstantBuffers>, ConstantPackRefusal> {
        if let Some(Some(existing)) = self.passes.get(pass_index) {
            prepare_cost.note_intern_hit();
            return Ok(Arc::clone(existing));
        }
        let key = packed_bank_key(
            executable.port,
            executable.local_banks,
            executable.code_constant_id,
        );
        let (packed, hit) = intern_packed_bank(&mut self.intern, self.intern_stamp, key, || {
            port.pack_hit(executable).map(Arc::new)
        })?;
        if hit {
            prepare_cost.note_intern_hit();
        } else {
            prepare_cost.note_intern_miss();
            prepare_cost.note_pack_seed(executable.local_banks.is_some());
        }
        if let Some(slot) = self.passes.get_mut(pass_index) {
            *slot = Some(Arc::clone(&packed));
        }
        Ok(packed)
    }
}

impl ExactPrepare<'_> {
    fn bind_hit_textures(
        &mut self,
        executable: ExecutablePassView<'_>,
        surface: super::SurfaceSamplerInputs,
        after_scene_resolve: bool,
    ) -> Result<Arc<[u32]>, GpuSubmitRefusal> {
        let port_gpu = self
            .pipeline_res
            .get(executable.port)
            .expect("exact port was admitted before texture binding");
        let key = BoundTextureKey {
            identity: TextureBindIdentity {
                port: executable.port,
                local: executable
                    .local_samplers
                    .map(|packed| packed.id)
                    .unwrap_or(0),
                code: executable.code_sampler_id,
                surface,
            },
            spot_shadow_select: self.spot_shadow_select,
        };
        let (texture_slots, table) = match &mut self.textures {
            PrepareTextureTables::Scene { slots, tables } => {
                let index = texture_table::scene_table_index(
                    after_scene_resolve,
                    GfxPassState::from_bits(executable.state).srgb_write_enable(),
                );
                (&mut slots[index], &mut tables[index])
            }
            PrepareTextureTables::Shadow { slots, table } => (&mut **slots, &mut **table),
        };
        if let Some(slots) = texture_slots.get(&key) {
            self.cost.tex_bind_hit_n = self.cost.tex_bind_hit_n.saturating_add(1);
            return Ok(Arc::clone(slots));
        }
        self.cost.tex_bind_miss_n = self.cost.tex_bind_miss_n.saturating_add(1);
        let textures = port_gpu
            .port
            .resolve_uploaded_texture_binds(
                executable,
                surface,
                &self.extracted.world.image_handles,
                self.extracted.world.generation,
                self.uploaded,
                self.sampler_table,
                self.spot_shadow_select,
            )
            .map_err(GpuSubmitRefusal::TextureBind)?;
        let slots = texture_slot_words(self.device, table, &textures)?;
        texture_slots.insert(key, Arc::clone(&slots));
        Ok(slots)
    }
}

fn texture_slot_words(
    device: &RenderDevice,
    table: &mut ExactTextureTable,
    textures: &[UploadedTextureBind],
) -> Result<Arc<[u32]>, GpuSubmitRefusal> {
    textures
        .iter()
        .map(|lane| {
            table
                .slot_word(device, lane)
                .map_err(GpuSubmitRefusal::TextureTable)
        })
        .collect::<Result<Vec<u32>, _>>()
        .map(Arc::from)
}

#[derive(Default)]
struct PrepareCost {
    pack_seed_n: u32,
    pack_walk_n: u32,
    tex_bind_hit_n: u32,
    tex_bind_miss_n: u32,
    intern_hit_n: u32,
    intern_miss_n: u32,
}

enum PrepareTextureTables<'a> {
    Scene {
        slots: &'a mut [HashMap<BoundTextureKey, Arc<[u32]>>; 4],
        tables: &'a mut [ExactTextureTable; 4],
    },
    Shadow {
        slots: &'a mut HashMap<BoundTextureKey, Arc<[u32]>>,
        table: &'a mut ExactTextureTable,
    },
}

struct ExactPrepare<'a> {
    extracted: ExtractedColourRefs<'a>,
    geometry: &'a ExactColourGeometry,

    pretess: Option<&'a CameraWorldPretess>,
    pipeline_res: &'a ExactColourPipeline,
    registry: &'a ExactPipelineRegistry,
    device: &'a RenderDevice,
    uploaded: &'a RuntimeUploadedImageRegistry,

    spot_shadow_select: Option<u8>,
    sampler_table: &'a RetailSamplerTable,

    textures: PrepareTextureTables<'a>,

    arena: Option<&'a mut ArenaPack>,
    run_pack: RunPackCache,
    cost: PrepareCost,
    skinned_tess: Option<&'a mut smodel_skinned::SmodelSkinnedTess>,
}

#[derive(Clone, Copy)]
struct ExactPrepareTarget {
    color: TextureFormat,
    samples: u32,
    depth: TextureFormat,
    forward_z: bool,
    use_world_pretess: bool,
}

#[derive(Clone, Copy)]
struct SunFlush {
    kind: SunFlushKind,
    draw_start: u32,
    draw_count: u32,
    ring_epoch: u32,
}

impl PrepareCost {
    fn note_pack_seed(&mut self, seeded: bool) {
        if seeded {
            self.pack_seed_n = self.pack_seed_n.saturating_add(1);
        } else {
            self.pack_walk_n = self.pack_walk_n.saturating_add(1);
        }
    }
    fn note_intern_hit(&mut self) {
        self.intern_hit_n = self.intern_hit_n.saturating_add(1);
    }
    fn note_intern_miss(&mut self) {
        self.intern_miss_n = self.intern_miss_n.saturating_add(1);
    }
}

#[derive(Default)]
struct WorldRunGather {
    indices: Vec<u32>,
    ranges: Vec<(u32, u32)>,

    index_gaps: u32,
}

fn empty_world_run_gather(ranges: Vec<(u32, u32)>, index_gaps: u32) -> WorldRunGather {
    WorldRunGather {
        indices: Vec::new(),
        ranges,
        index_gaps,
    }
}

fn world_submit_ranges_for_pass<'a>(
    geometry: &'a ExactColourGeometry,
    pretess: Option<&'a CameraWorldPretess>,
    use_world_pretess: bool,
) -> &'a [(u32, u32)] {
    match pretess.filter(|_| use_world_pretess) {
        Some(pretess) => pretess.ranges(),
        None => &geometry.world_surface_ranges,
    }
}

fn gather_world_run_indices(
    hits: impl IntoIterator<Item = (u16, u64, super::SurfaceSamplerInputs)>,
    src_indices: &[u32],
    src_ranges: &[(u32, u32)],
) -> WorldRunGather {
    let mut ranges = vec![(0, 0); src_ranges.len()];
    let mut indices = Vec::new();
    let mut prev: Option<(u64, super::SurfaceSamplerInputs, u32)> = None;
    let mut index_gaps = 0u32;
    for (surf, key, samplers) in hits {
        let Some(&(start, count)) = src_ranges.get(usize::from(surf)) else {
            prev = None;
            continue;
        };
        if count == 0 {
            prev = None;
            continue;
        }
        let dest = indices.len() as u32;
        let Some(slice) = src_indices.get(start as usize..(start.saturating_add(count)) as usize)
        else {
            prev = None;
            continue;
        };
        if let Some((prev_key, prev_samplers, end)) = prev
            && prev_key == key
            && prev_samplers == samplers
            && end != start
        {
            index_gaps = index_gaps.saturating_add(1);
        }
        indices.extend_from_slice(slice);
        if let Some(slot) = ranges.get_mut(usize::from(surf)) {
            *slot = (dest, count);
        }
        prev = Some((key, samplers, start.saturating_add(count)));
    }
    WorldRunGather {
        indices,
        ranges,
        index_gaps,
    }
}

fn colour_tech_at(
    colour: &FrameProduct,
    light: &FrameProduct,
    emissive: &FrameProduct,
    di: usize,
) -> Option<TechType> {
    let colour_n = colour.ordered_draws.len();
    let light_n = light.ordered_draws.len();
    if di < colour_n {
        colour.draw_tech.get(di).copied()
    } else if di < colour_n + light_n {
        light.draw_tech.get(di - colour_n).copied()
    } else {
        emissive.draw_tech.get(di - colour_n - light_n).copied()
    }
}

fn colour_draw_at<'a>(
    colour: &'a FrameProduct,
    light: &'a FrameProduct,
    emissive: &'a FrameProduct,
    di: usize,
) -> Option<&'a RetainedDrawItem> {
    let colour_n = colour.ordered_draws.len();
    let light_n = light.ordered_draws.len();
    if di < colour_n {
        colour.ordered_draws.get(di)
    } else if di < colour_n + light_n {
        light.ordered_draws.get(di - colour_n)
    } else {
        emissive.ordered_draws.get(di - colour_n - light_n)
    }
}

fn packed_draw_index(packed: &render_frame::PackedFrontendLists, emit: PackedEmit) -> Option<u32> {
    let i = emit.entry as usize;
    match emit.list {
        PackedListKind::World => packed.world_draw_indices.get(i).copied(),
        PackedListKind::XModel => packed.xmodel_draw_indices.get(i).copied(),
        PackedListKind::Smodel => packed.smodel_draw_indices.get(i).copied(),
        PackedListKind::Cached => packed.smodel_cached_draw_indices.get(i).copied(),
        PackedListKind::Pretess => packed.smodel_pretess_draw_indices.get(i).copied(),
        PackedListKind::SmodelSkinned => packed.smodel_skinned_draw_indices.get(i).copied(),
    }
}

struct WorldPackedSurf<'a> {
    item: &'a RetainedDrawItem,
    world_surf: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorldPackedRowMeta {
    di: u32,
    world_surf: Option<u16>,
}

impl WorldPackedSurf<'_> {
    fn world_surf(&self) -> Option<u16> {
        self.world_surf.or_else(|| match self.item.kind {
            RetainedDrawKind::World { surf, .. } => Some(surf),
            _ => None,
        })
    }
}

fn world_packed_row_meta(
    colour: &FrameProduct,
    light: &FrameProduct,
    emissive: &FrameProduct,
    packed: &render_frame::PackedFrontendLists,
    table: &[u16],
) -> Vec<Option<WorldPackedRowMeta>> {
    let skip_end = colour
        .ordered_draws
        .len()
        .saturating_add(light.ordered_draws.len())
        .saturating_add(emissive.ordered_draws.len());
    let mut prior = vec![0u16; skip_end];
    let mut rows = Vec::with_capacity(packed.world_draw_indices.len());
    for &di in &packed.world_draw_indices {
        let di_us = di as usize;
        let Some(item) = colour_draw_at(colour, light, emissive, di_us) else {
            rows.push(None);
            continue;
        };
        let seen = prior.get(di_us).copied().unwrap_or(0);
        let world_surf = match item.kind {
            RetainedDrawKind::World {
                surf, run, run_off, ..
            } if run > 1 => Some(
                table
                    .get(run_off as usize + usize::from(seen))
                    .copied()
                    .unwrap_or(surf),
            ),
            _ => None,
        };
        if let Some(slot) = prior.get_mut(di_us) {
            *slot = seen.saturating_add(1);
        }
        rows.push(Some(WorldPackedRowMeta { di, world_surf }));
    }
    rows
}

fn bind_world_packed_rows<'a>(
    colour: &'a FrameProduct,
    light: &'a FrameProduct,
    emissive: &'a FrameProduct,
    rows: &'a [Option<WorldPackedRowMeta>],
) -> impl Iterator<Item = Option<WorldPackedSurf<'a>>> + 'a {
    rows.iter().map(move |meta| {
        let meta = meta.as_ref()?;
        colour_draw_at(colour, light, emissive, meta.di as usize).map(|item| WorldPackedSurf {
            item,
            world_surf: meta.world_surf,
        })
    })
}

struct PreparedColourRow<'a> {
    item: &'a RetainedDrawItem,
    world_surf: Option<u16>,
}

impl PreparedColourRow<'_> {
    fn expanded_item(&self) -> Option<RetainedDrawItem> {
        let surf = self.world_surf?;
        let RetainedDrawKind::World {
            surf: first_surf,
            world_from_local,
            bsp_kind,
            bsp_run_first,
            setup_key_changed,
            ..
        } = self.item.kind
        else {
            return None;
        };
        let mut item = *self.item;
        item.kind = match bsp_kind {
            Some(kind) => RetainedDrawKind::World {
                surf,
                run: 1,
                run_off: 0,
                bsp_kind: Some(kind),
                bsp_run_first: Some(
                    bsp_run_first.expect("BSP world draw is missing its producer run identity"),
                ),
                setup_key_changed: setup_key_changed && surf == first_surf,
                world_from_local: Mat4::IDENTITY,
            },
            None => RetainedDrawKind::world_with_pose(surf, world_from_local),
        };
        Some(item)
    }
}

fn colour_entry_index_span(
    packed: &render_frame::PackedFrontendLists,
    emit: PackedEmit,
) -> Option<(u32, u32)> {
    let index = emit.entry as usize;
    if emit.list == PackedListKind::XModel {
        return packed
            .xmodel
            .get(index)
            .map(|entry| (entry.index_byte_offset / 2, u32::from(entry.tri_count) * 3));
    }
    let entries = match emit.list {
        PackedListKind::Smodel => &packed.smodel,
        PackedListKind::Cached => &packed.smodel_cached,
        PackedListKind::Pretess => &packed.smodel_pretess,
        PackedListKind::SmodelSkinned | PackedListKind::World | PackedListKind::XModel => {
            return None;
        }
    };
    entries
        .get(index)
        .map(|entry| (entry.index_byte_offset / 2, u32::from(entry.tri_count) * 3))
}

fn intended_colour_tech(
    colour: &FrameProduct,
    light: &FrameProduct,
    emissive: &FrameProduct,
    di: usize,
) -> TechType {
    colour_tech_at(colour, light, emissive, di).unwrap_or(TechType(0))
}

fn build_colour_row_plan(
    colour: &FrameProduct,
    light: &FrameProduct,
    emissive: &FrameProduct,
    packed: &render_frame::PackedFrontendLists,
    work: &super::backend::ColourDrawListWork,
    world_run_surfs: &[u16],
    pretess: &CameraWorldPretess,
) -> Vec<ColourRowPlan> {
    let layout_epoch = pretess.layout_epoch();
    let skip_end = colour
        .ordered_draws
        .len()
        .saturating_add(light.ordered_draws.len())
        .saturating_add(emissive.ordered_draws.len());
    let mut emitted = vec![false; skip_end];
    let mut rows = Vec::with_capacity(work.emit_order.len().max(skip_end));
    for emit in &work.emit_order {
        let Some(di) = packed_draw_index(packed, *emit) else {
            continue;
        };
        let already_emitted = emitted.get(di as usize).copied().unwrap_or(false);
        if emit.list == PackedListKind::World && already_emitted {
            continue;
        }
        if let Some(slot) = emitted.get_mut(di as usize) {
            *slot = true;
        }
        let tech = intended_colour_tech(colour, light, emissive, di as usize);
        if emit.list == PackedListKind::World {
            let Some(item) = colour_draw_at(colour, light, emissive, di as usize) else {
                continue;
            };
            if let Some(index_span) = world_emit_run_dest_span(item, world_run_surfs, pretess) {
                rows.push(ColourRowPlan {
                    draw_index: di,
                    world_surface_override: None,
                    technique: tech,
                    index_span: Some(index_span),
                    layout_epoch,
                });
            } else {
                push_expanded_world_row_plan(
                    item,
                    di,
                    tech,
                    world_run_surfs,
                    layout_epoch,
                    &mut rows,
                );
            }
            continue;
        }
        let span = colour_entry_index_span(packed, *emit);
        if colour_draw_at(colour, light, emissive, di as usize).is_none() {
            continue;
        }
        rows.push(ColourRowPlan {
            draw_index: di,
            world_surface_override: None,
            technique: tech,
            index_span: span,
            layout_epoch,
        });
    }
    for i in 0..skip_end {
        if emitted.get(i).copied().unwrap_or(false) {
            continue;
        }
        let Some(item) = colour_draw_at(colour, light, emissive, i) else {
            continue;
        };
        let tech = intended_colour_tech(colour, light, emissive, i);
        push_expanded_world_row_plan(
            item,
            i as u32,
            tech,
            world_run_surfs,
            layout_epoch,
            &mut rows,
        );
    }
    rows
}

fn world_emit_run_dest_span(
    item: &RetainedDrawItem,
    table: &[u16],
    pretess: &CameraWorldPretess,
) -> Option<(u32, u32)> {
    let RetainedDrawKind::World {
        surf, run, run_off, ..
    } = item.kind
    else {
        return None;
    };
    let run = run.max(1);
    let mut span: Option<(u32, u32)> = None;
    for offset in 0..run {
        let surf = if run == 1 {
            surf
        } else {
            *table.get(run_off as usize + usize::from(offset))?
        };
        let &(start, count) = pretess.ranges().get(usize::from(surf))?;
        if count == 0 {
            return None;
        }
        span = Some(match span {
            None => (start, count),
            Some((span_start, span_count)) => {
                if span_start.saturating_add(span_count) != start {
                    return None;
                }
                (span_start, span_count.saturating_add(count))
            }
        });
    }
    span
}

fn push_expanded_world_row_plan(
    item: &RetainedDrawItem,
    draw_index: u32,
    tech: TechType,
    table: &[u16],
    layout_epoch: u64,
    out: &mut Vec<ColourRowPlan>,
) {
    if let RetainedDrawKind::World { run, run_off, .. } = item.kind
        && run > 1
        && let Some(surfs) = table.get(run_off as usize..run_off as usize + usize::from(run))
        && surfs.len() == usize::from(run)
    {
        for &s in surfs {
            out.push(ColourRowPlan {
                draw_index,
                world_surface_override: Some(s),
                technique: tech,
                index_span: None,
                layout_epoch,
            });
        }
        return;
    }
    out.push(ColourRowPlan {
        draw_index,
        world_surface_override: None,
        technique: tech,
        index_span: None,
        layout_epoch,
    });
}

fn world_material_sorted(packed: u64) -> u16 {
    dpvs_iw4::GfxDrawSurf { packed }.material_sorted_index()
}

struct WorldPretessLayout {
    key: WorldPretessKey,
    index: Option<Buffer>,
    ranges: Vec<(u32, u32)>,
    index_gaps: u32,
    logical_index_count: u32,
    epoch: u64,
}

#[derive(Resource, Default)]
struct CameraWorldPretess {
    layout: Option<WorldPretessLayout>,

    epoch: u64,
}

impl CameraWorldPretess {
    fn ranges(&self) -> &[(u32, u32)] {
        self.layout
            .as_ref()
            .map(|layout| layout.ranges.as_slice())
            .unwrap_or(&[])
    }

    fn index(&self) -> Option<&Buffer> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.index.as_ref())
    }

    fn logical_index_count(&self) -> u32 {
        self.layout
            .as_ref()
            .map(|layout| layout.logical_index_count)
            .unwrap_or(0)
    }

    fn layout_epoch(&self) -> u64 {
        self.layout.as_ref().map(|layout| layout.epoch).unwrap_or(0)
    }
}

fn exact_draw_run_continues(prev: &PreparedExactDraw, next: &PreparedExactDraw) -> bool {
    prev.after_scene_resolve == next.after_scene_resolve
        && prev.tess == next.tess
        && prev.pipeline == next.pipeline
        && prev.port == next.port
        && same_packed_constants(prev, next)
        && same_texture_slots(prev, next)
        && prev.depth_min == next.depth_min
        && prev.depth_max == next.depth_max
        && prev.smc_stream_off == next.smc_stream_off
        && bsp_prepared_runs_continue(prev, next)
        && prev.start.saturating_add(prev.count) == next.start
}

fn bsp_prepared_runs_continue(prev: &PreparedExactDraw, next: &PreparedExactDraw) -> bool {
    bsp_run_identity_continues(
        prev.bsp_kind,
        prev.bsp_run_first,
        prev.bsp_first_surf,
        prev.bsp_surf_count,
        next.bsp_kind,
        next.bsp_run_first,
        next.bsp_first_surf,
    )
}

fn bsp_run_identity_continues(
    previous_kind: Option<BspCameraLane>,
    previous_run_first: u16,
    previous_first: u16,
    previous_count: u16,
    current_kind: Option<BspCameraLane>,
    current_run_first: u16,
    current_first: u16,
) -> bool {
    match (previous_kind, current_kind) {
        (None, None) => true,
        (Some(previous), Some(current)) => {
            previous == current
                && previous_run_first == current_run_first
                && previous_first.checked_add(previous_count) == Some(current_first)
        }
        _ => false,
    }
}

fn same_texture_slots(prev: &PreparedExactDraw, next: &PreparedExactDraw) -> bool {
    Arc::ptr_eq(&prev.texture_slots, &next.texture_slots)
        || prev.texture_slots == next.texture_slots
}

fn same_packed_constants(prev: &PreparedExactDraw, next: &PreparedExactDraw) -> bool {
    match (&prev.constants, &next.constants) {
        (Some(left), Some(right)) => Arc::ptr_eq(left, right) || left == right,
        (None, None) => prev.constant_base.is_some() && prev.constant_base == next.constant_base,
        _ => false,
    }
}

fn coalesce_indexed_runs<T>(
    draws: &mut [T],
    can_extend: impl Fn(&T, &T) -> bool,
    extend: impl Fn(&mut T, &T),
) -> usize {
    let mut write = 0usize;
    let mut read = 0usize;
    while read < draws.len() {
        if write > 0 && can_extend(&draws[write - 1], &draws[read]) {
            let (head, tail) = draws.split_at_mut(read);
            extend(&mut head[write - 1], &tail[0]);
        } else {
            if write != read {
                draws.swap(write, read);
            }
            write += 1;
        }
        read += 1;
    }
    write
}

fn tess_flush_merge_slot<T>(
    out: &[T],
    draw: &T,
    continues: impl Fn(&T, &T) -> bool,
    look_past_sibling_pass: impl Fn(&T, &T) -> bool,
) -> Option<usize> {
    let mut i = out.len();
    while i > 0 {
        i -= 1;
        if continues(&out[i], draw) {
            return Some(i);
        }
        if look_past_sibling_pass(&out[i], draw) {
            continue;
        }
        break;
    }
    None
}

fn colour_look_past_sibling_pass(prev: &PreparedExactDraw, next: &PreparedExactDraw) -> bool {
    prev.after_scene_resolve == next.after_scene_resolve
        && prev.tess == next.tess
        && prev.pipeline != next.pipeline
        && (prev.start.saturating_add(prev.count) == next.start
            || (prev.start == next.start && prev.count == next.count))
}

fn coalesce_exact_draws(draws: &mut Vec<PreparedExactDraw>) {
    let mut len = 0usize;
    let mut read = 0usize;
    while read < draws.len() {
        let slot = {
            let (head, tail) = draws.split_at_mut(read);
            tess_flush_merge_slot(
                &head[..len],
                &tail[0],
                exact_draw_run_continues,
                colour_look_past_sibling_pass,
            )
        };
        match slot {
            Some(i) => {
                let count = draws[read].count;
                let bsp_surf_count = draws[read].bsp_surf_count;
                let bsp_counted = draws[read].bsp_counted;
                draws[i].count = draws[i].count.saturating_add(count);
                draws[i].bsp_surf_count = draws[i].bsp_surf_count.saturating_add(bsp_surf_count);
                draws[i].bsp_counted |= bsp_counted;
                draws[i].binds_sun_shadow |= draws[read].binds_sun_shadow;
                draws[i].binds_spot_shadow |= draws[read].binds_spot_shadow;
            }
            None => {
                if len != read {
                    draws.swap(len, read);
                }
                len += 1;
            }
        }
        read += 1;
    }
    draws.truncate(len);
}

fn shadow_gpu_state_continues(
    same_pipeline: bool,
    same_tess: bool,
    same_epoch: bool,
    same_texture: bool,
    prev_base: Option<u32>,
    next_base: Option<u32>,
    prev_depth: (f32, f32),
    next_depth: (f32, f32),
    prev_start: u32,
    prev_count: u32,
    next_start: u32,
) -> bool {
    same_pipeline
        && same_tess
        && same_epoch
        && same_texture
        && prev_base.is_some()
        && prev_base == next_base
        && prev_depth == next_depth
        && prev_start.saturating_add(prev_count) == next_start
}

fn shadow_draw_run_continues(prev: &PreparedExactDraw, next: &PreparedExactDraw) -> bool {
    shadow_gpu_state_continues(
        prev.pipeline == next.pipeline && prev.port == next.port,
        prev.tess == next.tess,
        prev.ring_epoch == next.ring_epoch,
        same_texture_slots(prev, next),
        prev.constant_base,
        next.constant_base,
        (prev.depth_min, prev.depth_max),
        (next.depth_min, next.depth_max),
        prev.start,
        prev.count,
        next.start,
    )
}

fn coalesce_shadow_draws(draws: &mut Vec<PreparedExactDraw>) {
    let live = coalesce_shadow_draws_prefix(draws);
    draws.truncate(live);
}

fn coalesce_shadow_draws_prefix(draws: &mut [PreparedExactDraw]) -> usize {
    coalesce_indexed_runs(draws, shadow_draw_run_continues, |prev, next| {
        prev.count = prev.count.saturating_add(next.count);
        prev.bsp_surf_count = prev.bsp_surf_count.saturating_add(next.bsp_surf_count);
        prev.bsp_counted |= next.bsp_counted;
    })
}

fn submit_exact_draws<'a>(
    encoder: &mut CommandEncoder,
    device: &RenderDevice,
    target: &ViewTarget,
    depth: &ViewDepthTexture,
    extracted_view: &ExtractedView,
    geometry: &ExactColourGeometry,
    smodel_cache_gpu: &SmodelCacheGpu,
    smodel_skinned_vertex: Option<&Buffer>,
    smodel_skinned_index: Option<&Buffer>,
    pretess: &CameraWorldPretess,
    indirect: &indirect::ExactIndirectDraws,
    registry: &ExactPipelineRegistry,
    constant_arena: &ExactConstantArena,
    textures_bind: [&BindGroup; 2],
    draws: impl IntoIterator<Item = &'a PreparedExactDraw>,
    label: &'static str,
    refused_draws: &mut u32,
    last_refusal: &mut Option<GpuSubmitRefusal>,
    encode_not_ready: &mut u32,
    focused_object_id: Option<u16>,
) -> (u32, f32, [u32; 4], [u32; 4], u32, RecordCensus) {
    let mut draws = draws.into_iter().peekable();
    if draws.peek().is_none() {
        return (0, 0.0, [0; 4], [0; 4], 0, RecordCensus::default());
    }
    let views = ExactColourTargetViews::new(target);
    let mut indexed = 0u32;
    let mut drop_ms = 0.0f32;
    let mut bsp_drawn_surfaces = [0u32; 4];
    let mut bsp_draw_refused_surfaces = [0u32; 4];
    let mut focused_drawn = 0u32;
    let mut record_n = RecordCensus::default();
    while let Some(first) = draws.peek() {
        let srgb_write = first.state.srgb_write_enable();
        let mut ops = target.get_color_attachment().ops;
        if srgb_write && matches!(ops.load, LoadOp::Clear(_)) {
            let (view, resolve_target) = views.attachment_views(false);
            let clear_attachments = [Some(RenderPassColorAttachment {
                view,
                resolve_target: resolve_target.map(|view| &**view),
                ops,
                depth_slice: None,
            })];
            let clear_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("iw4_exact_colour_raw_clear"),
                color_attachments: &clear_attachments,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            drop(clear_pass);
            ops.load = LoadOp::Load;
        }
        let (view, resolve_target) = views.attachment_views(srgb_write);
        let attachment = RenderPassColorAttachment {
            view,
            resolve_target: resolve_target.map(|view| &**view),
            ops,
            depth_slice: None,
        };
        let (run_indexed, run_drop_ms, run_drawn, run_refused, run_focused_drawn, run_binds) =
            submit_exact_draw_run(
                encoder,
                device,
                attachment,
                depth,
                extracted_view,
                geometry,
                smodel_cache_gpu,
                smodel_skinned_vertex,
                smodel_skinned_index,
                pretess,
                indirect,
                registry,
                constant_arena,
                textures_bind[usize::from(srgb_write)],
                std::iter::from_fn(|| {
                    draws.next_if(|draw| draw.state.srgb_write_enable() == srgb_write)
                }),
                label,
                refused_draws,
                last_refusal,
                encode_not_ready,
                focused_object_id,
            );
        indexed = indexed.saturating_add(run_indexed);
        focused_drawn = focused_drawn.saturating_add(run_focused_drawn);
        record_n.add(run_binds);
        drop_ms += run_drop_ms;
        for lane in 0..3 {
            bsp_drawn_surfaces[lane] = bsp_drawn_surfaces[lane].saturating_add(run_drawn[lane]);
            bsp_draw_refused_surfaces[lane] =
                bsp_draw_refused_surfaces[lane].saturating_add(run_refused[lane]);
        }
    }
    (
        indexed,
        drop_ms,
        bsp_drawn_surfaces,
        bsp_draw_refused_surfaces,
        focused_drawn,
        record_n,
    )
}

fn submit_exact_draw_run<'a>(
    encoder: &mut CommandEncoder,
    device: &RenderDevice,
    attachment: RenderPassColorAttachment<'_>,
    depth: &ViewDepthTexture,
    extracted_view: &ExtractedView,
    geometry: &ExactColourGeometry,
    smodel_cache_gpu: &SmodelCacheGpu,
    smodel_skinned_vertex: Option<&Buffer>,
    smodel_skinned_index: Option<&Buffer>,
    pretess: &CameraWorldPretess,
    indirect: &indirect::ExactIndirectDraws,
    registry: &ExactPipelineRegistry,
    constant_arena: &ExactConstantArena,
    textures_bind: &BindGroup,
    draws: impl IntoIterator<Item = &'a PreparedExactDraw>,
    label: &'static str,
    refused_draws: &mut u32,
    last_refusal: &mut Option<GpuSubmitRefusal>,
    encode_not_ready: &mut u32,
    focused_object_id: Option<u16>,
) -> (u32, f32, [u32; 4], [u32; 4], u32, RecordCensus) {
    let attachments = [Some(attachment)];
    let mut pass = TrackedRenderPass::new(
        device,
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        }),
    );
    let mut bound_pipeline = None;
    let mut bound_tess = None;
    let mut bound_smc_off = None;
    let mut bound_depth = None;
    let mut constants_bound = false;

    let indirect_args = indirect.buffer();
    let mut batch = indirect::IndirectBatch::default();
    let mut record_n = RecordCensus::default();

    macro_rules! issue_indirect_batch {
        () => {
            if let Some(done) = batch.take() {
                issue_indirect(&mut pass, indirect_args, done, &mut record_n);
            }
        };
    }
    let mut indexed = 0u32;
    let mut bsp_drawn_surfaces = [0u32; 4];
    let mut bsp_draw_refused_surfaces = [0u32; 4];
    let mut focused_drawn = 0u32;
    let viewport = extracted_view.viewport;
    let vp_x = viewport.x as f32;
    let vp_y = viewport.y as f32;
    let vp_w = viewport.z as f32;
    let vp_h = viewport.w as f32;

    for draw in draws {
        let Some(gpu_pipeline) = registry.ready(draw.pipeline) else {
            *refused_draws = refused_draws.saturating_add(1);
            *encode_not_ready = encode_not_ready.saturating_add(1);
            *last_refusal = Some(GpuSubmitRefusal::PipelineNotReady);
            if draw.bsp_counted
                && let Some(kind) = draw.bsp_kind
            {
                let lane = bsp_kind_index(kind);
                bsp_draw_refused_surfaces[lane] =
                    bsp_draw_refused_surfaces[lane].saturating_add(u32::from(draw.bsp_surf_count));
            }
            continue;
        };
        if draw.count == 0 {
            continue;
        }
        let (Some(constant_base), Some(constants_bind)) =
            (draw.constant_base, constant_arena.gpu.bind_group.as_ref())
        else {
            *refused_draws = refused_draws.saturating_add(1);
            *last_refusal = Some(GpuSubmitRefusal::ConstantArenaMissing);
            continue;
        };
        let (vertex, index) = match draw.tess {
            ExactTessBind::World => (geometry.world_vertex.as_ref(), pretess.index()),
            ExactTessBind::Smodel => (
                geometry.smodel_vertex.as_ref(),
                geometry.smodel_index.as_ref(),
            ),
            ExactTessBind::SmodelCached => (
                smodel_cache_gpu.vertex_buffer(),
                smodel_cache_gpu.dynamic_index_buffer(),
            ),
            ExactTessBind::SmodelSkinned => (smodel_skinned_vertex, smodel_skinned_index),
            ExactTessBind::XModel => (
                geometry.xmodel.vertex.buffer(),
                geometry.xmodel.index.buffer(),
            ),
            ExactTessBind::CodeMesh => (geometry.fx_vertex.as_ref(), geometry.fx_index.as_ref()),
            ExactTessBind::ParticleCloud => (
                geometry.particle_cloud_vertex.as_ref(),
                geometry.particle_cloud_index.as_ref(),
            ),
            ExactTessBind::MarkMesh => (
                geometry.mark_mesh.vertex.buffer(),
                geometry.mark_mesh.index.buffer(),
            ),
            ExactTessBind::Glass => (
                geometry.glass_mesh.vertex.buffer(),
                geometry.glass_mesh.index.buffer(),
            ),
        };
        let (Some(vertex), Some(index)) = (vertex, index) else {
            *refused_draws = refused_draws.saturating_add(1);
            *last_refusal = Some(GpuSubmitRefusal::UnsupportedTessKind);
            if draw.bsp_counted
                && let Some(kind) = draw.bsp_kind
            {
                let lane = bsp_kind_index(kind);
                bsp_draw_refused_surfaces[lane] =
                    bsp_draw_refused_surfaces[lane].saturating_add(u32::from(draw.bsp_surf_count));
            }
            continue;
        };
        if bound_pipeline != Some(draw.pipeline)
            || bound_tess != Some(draw.tess)
            || (draw.tess == ExactTessBind::SmodelCached
                && bound_smc_off != Some(draw.smc_stream_off))
        {
            issue_indirect_batch!();
            pass.set_render_pipeline(gpu_pipeline);
            let vertex_slice = if draw.tess == ExactTessBind::SmodelCached {
                let end = draw
                    .smc_stream_off
                    .saturating_add(u64::from(lighting_iw4::SMC_BANK_VB_BYTES));
                vertex.slice(draw.smc_stream_off..end)
            } else {
                vertex.slice(..)
            };
            pass.set_vertex_buffer(0, vertex_slice);
            record_n.state = record_n.state.saturating_add(2);
            if draw.tess == ExactTessBind::World {
                if let Some(layer) = geometry.world_layer.as_ref() {
                    pass.set_vertex_buffer(1, layer.slice(..));
                    record_n.state = record_n.state.saturating_add(1);
                }
            }
            let index_format = if matches!(
                draw.tess,
                ExactTessBind::SmodelCached | ExactTessBind::MarkMesh
            ) {
                IndexFormat::Uint16
            } else {
                IndexFormat::Uint32
            };
            pass.set_index_buffer(index.slice(..), index_format);
            record_n.state = record_n.state.saturating_add(1);
            bound_pipeline = Some(draw.pipeline);
            bound_tess = Some(draw.tess);
            bound_smc_off = Some(draw.smc_stream_off);
        }
        if vp_w > 0.0 && vp_h > 0.0 && bound_depth != Some((draw.depth_min, draw.depth_max)) {
            issue_indirect_batch!();
            pass.set_viewport(vp_x, vp_y, vp_w, vp_h, draw.depth_min, draw.depth_max);
            record_n.state = record_n.state.saturating_add(1);
            bound_depth = Some((draw.depth_min, draw.depth_max));
        }

        if !constants_bound {
            pass.set_bind_group(0, constants_bind, &[]);
            pass.set_bind_group(1, textures_bind, &[]);
            constants_bound = true;
            record_n.group0 = record_n.group0.saturating_add(1);
            record_n.group1 = record_n.group1.saturating_add(1);
        }
        if draw.tess == ExactTessBind::World {
            let logical = pretess.logical_index_count();
            let end = draw.start.saturating_add(draw.count);
            if end > logical {
                *refused_draws = refused_draws.saturating_add(1);
                *last_refusal = Some(GpuSubmitRefusal::WorldPretessSpanBeyondLimit {
                    start: draw.start,
                    count: draw.count,
                    logical_len: logical,
                    epoch: pretess.layout_epoch(),
                });
                continue;
            }
        }
        match (indirect_args, draw.indirect_arg) {
            (Some(_), Some(slot)) => {
                if let Some(done) = batch.push(slot) {
                    issue_indirect(&mut pass, indirect_args, done, &mut record_n);
                }
            }
            _ => pass.draw_indexed(
                draw.start..draw.start.saturating_add(draw.count),
                0,
                constant_base..constant_base.saturating_add(1),
            ),
        }
        indexed = indexed.saturating_add(1);
        if draw.owner_object_id == focused_object_id && focused_object_id.is_some() {
            focused_drawn = focused_drawn.saturating_add(1);
        }
        if draw.bsp_counted
            && let Some(kind) = draw.bsp_kind
        {
            let lane = bsp_kind_index(kind);
            bsp_drawn_surfaces[lane] =
                bsp_drawn_surfaces[lane].saturating_add(u32::from(draw.bsp_surf_count));
        }
    }
    issue_indirect_batch!();
    let drop_started = Instant::now();
    drop(pass);
    (
        indexed,
        drop_started.elapsed().as_secs_f32() * 1000.0,
        bsp_drawn_surfaces,
        bsp_draw_refused_surfaces,
        focused_drawn,
        record_n,
    )
}

fn issue_indirect<'a>(
    pass: &mut TrackedRenderPass<'a>,
    args: Option<&'a Buffer>,
    batch: indirect::IndirectBatch,
    record_n: &mut RecordCensus,
) {
    let Some(args) = args else {
        return;
    };
    pass.multi_draw_indexed_indirect(
        args,
        indirect::ExactIndirectDraws::byte_offset(batch.first()),
        batch.count(),
    );
    record_n.multi_draws = record_n.multi_draws.saturating_add(1);
    record_n.multi_draw_commands = record_n.multi_draw_commands.saturating_add(batch.count());
}

fn append_with_rollback<T, E>(
    out: &mut Vec<T>,
    write: impl FnOnce(&mut Vec<T>) -> Result<(), E>,
) -> Result<usize, E> {
    let start = out.len();
    match write(out) {
        Ok(()) => Ok(out.len().saturating_sub(start)),
        Err(cause) => {
            out.truncate(start);
            Err(cause)
        }
    }
}

impl ExactPrepare<'_> {
    fn prepare_ready_hit(
        &mut self,
        item: &RetainedDrawItem,
        execution: &MaterialExecution,
        run_serial: u64,
        place_code: Option<PlaceLanes<'_>>,
        surface: super::SurfaceSamplerInputs,
        target: ExactPrepareTarget,
        binds_sun_shadow: bool,
        binds_spot_shadow: bool,
        out: &mut Vec<PreparedExactDraw>,
    ) -> Result<usize, GpuSubmitRefusal> {
        let after_scene_resolve = matches!(self.textures, PrepareTextureTables::Scene { .. })
            && (item.camera_region == Some(asset_iw4::CAMERA_REGION_EMISSIVE)
                || matches!(item.kind, RetainedDrawKind::CodeMesh { .. })
                || matches!(item.kind, RetainedDrawKind::Glass { .. })
                || matches!(item.kind, RetainedDrawKind::MarkMesh { glass: true, .. })
                || execution_binds_code_texture(execution, CODE_TEXTURE_RESOLVED_POST_SUN)
                || execution_binds_code_texture(execution, CODE_TEXTURE_FLOATZ));
        let kind = item.kind;
        let key = item.key;
        let (tess, start, count, vertex_type) = match kind {
            RetainedDrawKind::World { surf, .. } => {
                if let Some(cause) = self.extracted.world.static_geometry.world_vertex_refusal {
                    return Err(GpuSubmitRefusal::WorldVertex(cause));
                }
                let &(start, count) = world_submit_ranges_for_pass(
                    self.geometry,
                    self.pretess,
                    target.use_world_pretess,
                )
                .get(usize::from(surf))
                .ok_or(GpuSubmitRefusal::MissingSurfaceRange { surf })?;
                if count == 0 {
                    return Err(GpuSubmitRefusal::EmptyIndexRange { surf });
                }
                if self.geometry.world_vertex.is_none() || self.geometry.world_index.is_none() {
                    return Err(GpuSubmitRefusal::WorldVertex(
                        render_frame::RetailWorldVertexRefusal::ForeignLayout {
                            source_layout: "exact colour world buffers absent",
                        },
                    ));
                }
                (ExactTessBind::World, start, count, execution.vertex_type)
            }
            RetainedDrawKind::Smodel {
                surface,
                placement,
                stream,
                cache_index,
                pretess,
                world_from_local,
                ..
            } => match stream {
                None => return Err(GpuSubmitRefusal::SmodelXSurfacePathUnread { placement }),
                Some(lighting_iw4::SmodelSurfPath::Cached) => {
                    if let Some(range) = pretess {
                        smodel_pretess_submit_refusal(
                            self.extracted,
                            placement,
                            cache_index
                                .ok_or(GpuSubmitRefusal::SmodelCacheIndexEmpty { placement })?,
                            range,
                        )?
                    } else {
                        return Err(GpuSubmitRefusal::SmodelCachedWithoutDestRange { placement });
                    }
                }
                Some(lighting_iw4::SmodelSurfPath::Pretess) => {
                    let cache_index =
                        cache_index.ok_or(GpuSubmitRefusal::SmodelCacheIndexEmpty { placement })?;
                    let pretess = pretess
                        .ok_or(GpuSubmitRefusal::SmodelCacheIndicesMissing { cache_index })?;
                    smodel_pretess_submit_refusal(self.extracted, placement, cache_index, pretess)?
                }

                Some(lighting_iw4::SmodelSurfPath::Rigid) => {
                    if let Some(cause) = self.extracted.world.static_geometry.smodel_vertex_refusal
                    {
                        return Err(GpuSubmitRefusal::PackedVertex(cause));
                    }
                    let &(start, count) = self
                        .geometry
                        .smodel_surface_ranges
                        .get(surface as usize)
                        .ok_or(GpuSubmitRefusal::MissingSmodelRange { surface })?;
                    if count == 0 {
                        return Err(GpuSubmitRefusal::EmptySmodelIndexRange { surface });
                    }
                    if self.geometry.smodel_vertex.is_none() || self.geometry.smodel_index.is_none()
                    {
                        return Err(GpuSubmitRefusal::PackedVertex(
                            render_frame::RetailPackedVertexRefusal::ForeignLayout {
                                source_layout: "exact colour smodel buffers absent",
                            },
                        ));
                    }
                    (
                        ExactTessBind::Smodel,
                        start,
                        count,
                        asset_iw4::vertex_decl::PACKED_VERTEX_TYPE,
                    )
                }
                Some(lighting_iw4::SmodelSurfPath::Skinned) => {
                    let tess = self
                        .skinned_tess
                        .as_mut()
                        .ok_or(GpuSubmitRefusal::SmodelSkinnedDestMissing { placement })?;
                    let (start, count) =
                        tess.append_draw(self.extracted, placement, surface, world_from_local)?;
                    if count == 0 {
                        return Err(GpuSubmitRefusal::EmptySmodelIndexRange { surface });
                    }
                    (
                        ExactTessBind::SmodelSkinned,
                        start,
                        count,
                        asset_iw4::vertex_decl::PACKED_VERTEX_TYPE,
                    )
                }
            },
            RetainedDrawKind::XModel { surface, .. } => {
                if let Some(cause) = self.extracted.frame.xmodel_vertex_refusal {
                    return Err(GpuSubmitRefusal::PackedVertex(cause));
                }
                let &(start, count) = self
                    .geometry
                    .xmodel_surface_ranges
                    .get(surface as usize)
                    .ok_or(GpuSubmitRefusal::MissingXModelRange { surface })?;
                if count == 0 {
                    return Err(GpuSubmitRefusal::EmptyXModelIndexRange { surface });
                }
                if !self.geometry.xmodel.drawable() {
                    return Err(GpuSubmitRefusal::PackedVertex(
                        render_frame::RetailPackedVertexRefusal::ForeignLayout {
                            source_layout: "exact colour xmodel buffers absent",
                        },
                    ));
                }
                (
                    ExactTessBind::XModel,
                    start,
                    count,
                    asset_iw4::vertex_decl::PACKED_VERTEX_TYPE,
                )
            }
            RetainedDrawKind::CodeMesh { draw, .. } => {
                if let Some(cause) = self.extracted.frame.fx_vertex_refusal {
                    return Err(GpuSubmitRefusal::PackedVertex(cause));
                }
                let &(start, count) = self
                    .geometry
                    .fx_surface_ranges
                    .get(draw as usize)
                    .ok_or(GpuSubmitRefusal::MissingCodeMeshRange { draw })?;
                if count == 0 {
                    return Err(GpuSubmitRefusal::EmptyCodeMeshIndexRange { draw });
                }
                if self.geometry.fx_vertex.is_none() || self.geometry.fx_index.is_none() {
                    return Err(GpuSubmitRefusal::PackedVertex(
                        render_frame::RetailPackedVertexRefusal::ForeignLayout {
                            source_layout: "exact colour code-mesh buffers absent",
                        },
                    ));
                }
                (
                    ExactTessBind::CodeMesh,
                    start,
                    count,
                    asset_iw4::vertex_decl::PACKED_VERTEX_TYPE,
                )
            }
            RetainedDrawKind::ParticleCloud { draw, .. } => {
                let &(start, count) = self
                    .geometry
                    .particle_cloud_surface_ranges
                    .get(draw as usize)
                    .ok_or(GpuSubmitRefusal::MissingParticleCloudRange { draw })?;
                if count == 0 {
                    return Err(GpuSubmitRefusal::EmptyParticleCloudIndexRange { draw });
                }
                if self.geometry.particle_cloud_vertex.is_none()
                    || self.geometry.particle_cloud_index.is_none()
                {
                    return Err(GpuSubmitRefusal::PackedVertex(
                        render_frame::RetailPackedVertexRefusal::ForeignLayout {
                            source_layout: "exact colour particle-cloud buffers absent",
                        },
                    ));
                }
                (
                    ExactTessBind::ParticleCloud,
                    start,
                    count,
                    asset_iw4::vertex_decl::POS_TEX_VERTEX_TYPE,
                )
            }
            RetainedDrawKind::MarkMesh { draw, packed, .. } => {
                let &(start, count) = self
                    .geometry
                    .mark_mesh_surface_ranges
                    .get(draw as usize)
                    .ok_or(GpuSubmitRefusal::MissingMarkMeshRange { draw })?;
                if count == 0 {
                    return Err(GpuSubmitRefusal::EmptyMarkMeshIndexRange { draw });
                }
                if !self.geometry.mark_mesh.drawable() {
                    return Err(GpuSubmitRefusal::WorldVertex(
                        render_frame::RetailWorldVertexRefusal::ForeignLayout {
                            source_layout: "exact colour mark-mesh buffers absent",
                        },
                    ));
                }
                (
                    ExactTessBind::MarkMesh,
                    start,
                    count,
                    if packed {
                        asset_iw4::vertex_decl::PACKED_VERTEX_TYPE
                    } else {
                        asset_iw4::vertex_decl::WORLD_VERTEX_TYPE
                    },
                )
            }
            RetainedDrawKind::Glass { draw, .. } => {
                if let Some(cause) = self.extracted.frame.glass_mesh_vertex_refusal {
                    return Err(GpuSubmitRefusal::PackedVertex(cause));
                }
                let &(start, count) = self
                    .geometry
                    .glass_mesh_surface_ranges
                    .get(draw as usize)
                    .ok_or(GpuSubmitRefusal::MissingGlassMeshRange { draw })?;
                if count == 0 {
                    return Err(GpuSubmitRefusal::EmptyGlassMeshIndexRange { draw });
                }
                if !self.geometry.glass_mesh.drawable() {
                    return Err(GpuSubmitRefusal::PackedVertex(
                        render_frame::RetailPackedVertexRefusal::ForeignLayout {
                            source_layout: "exact colour glass-mesh buffers absent",
                        },
                    ));
                }
                (
                    ExactTessBind::Glass,
                    start,
                    count,
                    asset_iw4::vertex_decl::PACKED_VERTEX_TYPE,
                )
            }
        };

        let smc_stream_off = if tess == ExactTessBind::SmodelCached {
            if let RetainedDrawKind::Smodel {
                cache_index: Some(cache_index),
                ..
            } = kind
            {
                u64::from(
                    lighting_iw4::r_smc_stream_source_byte_offset(cache_index)
                        .ok_or(GpuSubmitRefusal::SmodelCacheIndicesMissing { cache_index })?,
                )
            } else {
                0
            }
        } else {
            0
        };

        let (bsp_kind, bsp_run_first, bsp_first_surf, bsp_surf_count) = bsp_draw_source(&kind);
        self.run_pack.begin(run_serial, execution.pass_count());
        append_with_rollback(out, |out| {
            for (pass_index, executable) in execution.iter_passes().enumerate() {
                debug_assert_eq!(
                    executable.port.vertex_type, vertex_type,
                    "ExecutablePass.port was admitted for a different tess vertex type"
                );
                let pipeline_res = self.pipeline_res;
                let (port_index, pipeline) = match self.run_pack.pipelines.get(pass_index).copied()
                {
                    Some(Some(hit)) => hit,
                    _ => {
                        let port_index = pipeline_res.by_id.get(&executable.port).copied().ok_or(
                            GpuSubmitRefusal::NoExactPort {
                                pair: executable.shader_pair,
                            },
                        )?;
                        let host_state = GfxPassState::from_bits(executable.state);
                        let state0 = host_state.apply_change_state_0_host(AlphaMode::Opaque, false);
                        let state1 = host_state.apply_change_state_1_host();
                        let color = exact_colour_target_format(target.color, state0.srgb_write);
                        let key = ExactColourPipelineKey {
                            target: color,
                            depth_format: target.depth,
                            samples: target.samples,
                            state0,
                            state1,
                            port: executable.port,
                            cached_lighting: false,
                            mark_mesh: tess == ExactTessBind::MarkMesh
                                && vertex_type == asset_iw4::vertex_decl::PACKED_VERTEX_TYPE,
                            forward_z: target.forward_z,
                        };

                        let Some(pipeline) = self.registry.slot(&key) else {
                            self.registry.discover(key);
                            return Err(GpuSubmitRefusal::PipelineNotReady);
                        };
                        if !self.registry.is_ready(pipeline) {
                            return Err(GpuSubmitRefusal::PipelineNotReady);
                        }
                        if let Some(slot) = self.run_pack.pipelines.get_mut(pass_index) {
                            *slot = Some((port_index, pipeline));
                        }
                        (port_index, pipeline)
                    }
                };
                let port_gpu = &pipeline_res.ports[port_index];
                let texture_slots =
                    self.bind_hit_textures(executable, surface, after_scene_resolve)?;
                // Shadow placement is applied after preparation, independently for each
                // light and object. Its unplaced banks belong to the material run.
                let unplaced_shadow = matches!(self.textures, PrepareTextureTables::Shadow { .. })
                    && place_code.is_none();
                let (constants, constant_base) =
                    if identity_placement_kind(&kind) || unplaced_shadow {
                        let constants = self
                            .run_pack
                            .pack(pass_index, &port_gpu.port, executable, &mut self.cost)
                            .map_err(GpuSubmitRefusal::ConstantPack)?;
                        match self.arena.as_deref_mut() {
                            Some(arena) => (None, Some(arena.base_for(&constants, &texture_slots))),
                            None => (Some(constants), None),
                        }
                    } else if let Some(arena) = self.arena.as_deref_mut() {
                        let overlay = place_code.and_then(|c| c.pass(pass_index)).unwrap_or(&[]);
                        let (base, interned) = arena
                            .append_hit(
                                &port_gpu.port,
                                executable,
                                overlay,
                                &texture_slots,
                                key,
                                pass_index as u32,
                            )
                            .map_err(GpuSubmitRefusal::ConstantPack)?;
                        if interned {
                            self.cost.note_intern_hit();
                        } else {
                            self.cost.note_intern_miss();
                            self.cost.note_pack_seed(true);
                        }
                        (None, Some(base))
                    } else {
                        let mut packed = port_gpu
                            .port
                            .pack_hit(executable)
                            .map_err(GpuSubmitRefusal::ConstantPack)?;
                        if let Some(overlay) = place_code.and_then(|c| c.pass(pass_index)) {
                            overlay_packed_code_on_banks(
                                &mut packed.vertex,
                                &mut packed.pixel,
                                overlay,
                            )
                            .map_err(GpuSubmitRefusal::ConstantPack)?;
                        }
                        self.cost.note_pack_seed(executable.local_banks.is_some());
                        (Some(Arc::new(packed)), None)
                    };
                let (depth_min, depth_max) = if target.forward_z {
                    (0.0, 1.0)
                } else {
                    reverse_z_viewport_depth(depth_range_type_for_draw(&kind, key))
                };
                out.push(PreparedExactDraw {
                    after_scene_resolve,
                    pipeline,
                    port: executable.port,
                    constants,
                    constant_base,
                    texture_slots,
                    start,
                    count,
                    tess,
                    owner_object_id: matches!(kind, RetainedDrawKind::XModel { .. })
                        .then(|| drawsurf_object_id(key)),
                    depth_min,
                    depth_max,
                    state: GfxPassState::from_bits(executable.state),
                    ring_epoch: 0,
                    smc_stream_off,
                    bsp_kind,
                    bsp_run_first,
                    bsp_first_surf,
                    bsp_surf_count,
                    bsp_counted: bsp_kind.is_some() && pass_index == 0,
                    binds_sun_shadow,
                    binds_spot_shadow,

                    indirect_arg: None,
                });
            }
            Ok(())
        })
    }
}

pub(super) fn register(app: &mut App) {
    super::floatz::register(app);
    let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) else {
        return;
    };
    render_app
        .init_resource::<InstalledRenderWorld>()
        .init_resource::<PublishedRenderFrame>()
        .init_resource::<ExactColourGeometry>()
        .init_resource::<super::resolved_scene::ResolvedScene>()
        .init_resource::<SmodelCacheGpu>()
        .init_resource::<ExactColourPipeline>()
        .init_resource::<ExactColourBindingCache>()
        .init_resource::<ExactShadowBindingCache>()
        .init_resource::<ExactTextureTable>()
        .init_resource::<ShadowTextureTable>()
        .init_resource::<SceneTextureTables>()
        .init_resource::<ExactPipelineKickCache>()
        .init_resource::<ExactPipelineRegistry>()
        .init_resource::<ExactConstantArena>()
        .init_resource::<ExactColourSubmitCensus>()
        .init_resource::<ShadowmapSunGpu>()
        .init_resource::<ShadowmapSpotGpu>()
        .init_resource::<ShadowmapSunArena>()
        .init_resource::<ShadowmapSpotArena>()
        .init_resource::<ColourSubmitScratch>()
        .init_resource::<ShadowSubmitScratch>()
        .init_resource::<CameraPrepareState>()
        .init_resource::<CameraWorldPretess>()
        .init_resource::<prepare_camera::InstalledColourPass>()
        .init_resource::<indirect::ExactIndirectDraws>()
        .init_resource::<ResidentShadowStaticDraws>()
        .add_systems(
            Render,
            (
                pipeline::init_or_update_pipeline.in_set(RenderSystems::PrepareAssets),
                geometry::upload_exact_geometry.in_set(RenderSystems::PrepareResources),
                pipeline::kick_extracted_colour_pipelines.in_set(RenderSystems::Prepare),
                prepare_camera::install_shared_colour_pass
                    .in_set(RenderSystems::Prepare)
                    .after(geometry::upload_exact_geometry)
                    .after(pipeline::kick_extracted_colour_pipelines)
                    .after(super::gpu_resources::prepare_uploaded_image_registry),
                prepare_camera::prepare_colour_lanes
                    .in_set(RenderSystems::Prepare)
                    .after(prepare_camera::install_shared_colour_pass),
            ),
        )
        .add_systems(
            Core3d,
            (
                record::draw_exact_colour
                    .in_set(Core3dSystems::MainPass)
                    .in_set(super::draw::ExactColourDrawSet)
                    .after(main_opaque_pass_3d),
                record::copy_submit_prepare_ms.after(super::draw::ExactColourDrawSet),
            ),
        );
}
