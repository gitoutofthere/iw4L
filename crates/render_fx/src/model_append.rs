use render_anim::{
    XMODEL_PACKED_EMPTY_PLAN, XMODEL_PACKED_UNAVAILABLE, append_mesh, install_retained_packed,
};
use render_scene::SmodelPassMaterial;

use crate::model_draw::{FxModelAssetDraw, FxModelDrawPlan};

pub fn append_fx_model_asset(
    plan: &mut FxModelDrawPlan,
    model_index: usize,
    lod: u8,
    surfaces: &[render_anim::fpv_pose::PosedModelSurface],
    materials: &[Option<SmodelPassMaterial>],
) -> Vec<(u32, u32)> {
    let mut asset_surfaces = Vec::new();
    let vertices_empty = plan.vertices.is_empty();
    let mut packed_ok = vertices_empty
        || matches!(
            plan.packed_vertices,
            assets::RetailPackedVertexPayload::Iw4(_)
        );
    let mut packed = match std::mem::replace(
        &mut plan.packed_vertices,
        assets::RetailPackedVertexPayload::Unavailable {
            source_layout: XMODEL_PACKED_UNAVAILABLE,
        },
    ) {
        assets::RetailPackedVertexPayload::Iw4(rows) => rows,
        assets::RetailPackedVertexPayload::Unavailable { .. } => Vec::new(),
    };
    for (surface, material) in surfaces.iter().zip(materials) {
        let Some(material) = material else { continue };
        let before = plan.vertices.len();
        let Some((start, count)) =
            append_mesh(&surface.mesh, &mut plan.vertices, &mut plan.indices)
        else {
            continue;
        };
        let decoded = plan.vertices.len() - before;
        if packed_ok && surface.packed_vertices.len() == decoded {
            packed.extend_from_slice(&surface.packed_vertices);
        } else {
            packed_ok = false;
            packed.clear();
        }
        let surface_index = plan.surface_ranges.len() as u32;
        plan.surface_ranges.push((start, count));
        let material_index = plan.materials.len() as u32;
        plan.materials.push(material.clone());
        asset_surfaces.push((surface_index, material_index));
    }
    plan.packed_vertices = install_retained_packed(
        packed_ok,
        packed,
        plan.vertices.len(),
        XMODEL_PACKED_EMPTY_PLAN,
        XMODEL_PACKED_UNAVAILABLE,
    );
    plan.assets.push(FxModelAssetDraw {
        model_index,
        lod,
        surfaces: asset_surfaces.clone(),
    });
    let topology = {
        let mut revisions = render_frame::SourceRevisions::default();
        revisions.set_topology_from(&plan.indices, &plan.surface_ranges, plan.vertices.len());
        revisions.topology
    };
    let mut rev = plan.revision;
    plan.revisions.topology = topology;
    plan.revisions.bump_packed_write(&mut rev);
    plan.revision = rev;
    asset_surfaces
}
