//! Add `ReflectQuery` to the app for all pre-existing bevy components.
use bevy::{
    core_pipeline::bloom::BloomSettings,
    core_pipeline::fxaa::Fxaa,
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    pbr::wireframe::Wireframe,
    pbr::*,
    prelude::*,
    render::camera::CameraRenderGraph,
    render::mesh::skinning::SkinnedMesh,
    render::primitives::{Aabb, CascadesFrusta, CubemapFrusta, Frustum},
    render::view::{ColorGrading, RenderLayers, VisibleEntities},
    sprite::{Anchor, Mesh2dHandle},
    text::Text2dBounds,
    ui::{FocusPolicy, RelativeCursorPosition},
    window::PrimaryWindow,
};

use crate::ReflectQuery;

macro_rules! register_reflect_query {
    ($registry:expr, $( $to_register:ty ),* $(,)?) => {
        $( $registry.register_type_data::<$to_register, ReflectQuery>() );*
    };
}

/// Add [`ReflectQuery`] registration for all base bevy components. _All_.
pub struct BaseReflectQueryPlugin;
impl Plugin for BaseReflectQueryPlugin {
    fn build(&self, app: &mut App) {
        add_all_reflect_query(app);
    }
}

fn add_all_reflect_query(app: &mut App) {
    #[rustfmt::skip]
    register_reflect_query![ app,
        Anchor, Mesh2dHandle, Sprite, TextureAtlasSprite, DebandDither, Tonemapping, Projection,
        Visibility, FocusPolicy, Interaction, ZIndex, Name, BloomSettings, Camera2d, Camera3d,
        Fxaa, DepthPrepass, NormalPrepass, Children, Parent, Camera, CameraRenderGraph,
        OrthographicProjection, PerspectiveProjection, SkinnedMesh, Aabb, CascadesFrusta,
        CubemapFrusta, Frustum, ColorGrading, ComputedVisibility, RenderLayers, VisibleEntities,
        Text2dBounds, Text, GlobalTransform, Transform, BackgroundColor, CalculatedClip,
        CalculatedSize, Node, RelativeCursorPosition, Style, UiImage, Button, Label, PrimaryWindow,
        Window, FogSettings, Cascades, NotShadowReceiver, ClusterConfig, CascadesVisibleEntities,
        NotShadowCaster, DirectionalLight, AlphaMode, Wireframe, GlobalTransform,
        CascadeShadowConfig, SpotLight, EnvironmentMapLight, PointLight, CubemapVisibleEntities,
        Node, Button,
    ];
}
