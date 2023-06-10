//! Add `ReflectQueryable` to the app for all pre-existing bevy components.

use bevy::prelude::{App, Plugin};

use crate::ReflectQueryable;

macro_rules! register_reflect_query {
    ($registry:expr, $( $to_register:ty ),* $(,)?) => {
        $( $registry.register_type_data::<$to_register, ReflectQueryable>() );*
    };
}

/// Add [`ReflectQueryable`] registration for all base bevy components. _All_.
pub struct BaseReflectQueryablePlugin;
impl Plugin for BaseReflectQueryablePlugin {
    fn build(&self, app: &mut App) {
        add_all_reflect_query(app);
    }
}

#[allow(unused)]
fn add_all_reflect_query(app: &mut App) {
    {
        use bevy::prelude::{Children, GlobalTransform, Name, Parent, Transform, Window};
        use bevy::window::PrimaryWindow;
        register_reflect_query!(
            app,
            Children,
            GlobalTransform,
            Name,
            Parent,
            // PrimaryWindow,
            Transform,
            Window,
        );
    }
    #[cfg(feature = "register_core_pipeline")]
    {
        use bevy::core_pipeline::{
            bloom::BloomSettings,
            fxaa::Fxaa,
            prelude::{Camera2d, Camera3d},
            prepass::{DepthPrepass, NormalPrepass},
            tonemapping::{DebandDither, Tonemapping},
        };
        register_reflect_query!(
            app,
            BloomSettings,
            Camera2d,
            Camera3d,
            DebandDither,
            DepthPrepass,
            // Fxaa,
            NormalPrepass,
            Tonemapping
        );
    }

    #[cfg(feature = "register_pbr")]
    {
        use bevy::pbr::{wireframe::Wireframe, *};
        register_reflect_query![
            app,
            Cascades,
            CascadeShadowConfig,
            CascadesVisibleEntities,
            ClusterConfig,
            CubemapVisibleEntities,
            DirectionalLight,
            EnvironmentMapLight,
            // FogSettings,
            // NotShadowCaster,
            // NotShadowReceiver,
            PointLight,
            SpotLight,
            // Wireframe,??
        ];
    }
    #[cfg(feature = "register_sprite")]
    {
        use bevy::sprite::{Anchor, Mesh2dHandle, Sprite, TextureAtlasSprite};
        register_reflect_query![
            app,
            Anchor,
            Mesh2dHandle,
            Sprite, /*TextureAtlasSprite*/
        ];
    }

    #[cfg(feature = "register_render")]
    {
        use bevy::render::{
            camera::CameraRenderGraph,
            mesh::skinning::SkinnedMesh,
            prelude::{
                Camera, ComputedVisibility, OrthographicProjection, PerspectiveProjection,
                Projection, Visibility,
            },
            primitives::{Aabb, CascadesFrusta, CubemapFrusta, Frustum},
            view::{ColorGrading, RenderLayers, VisibleEntities},
        };
        register_reflect_query![
            app,
            Aabb,
            Camera,
            CameraRenderGraph,
            CascadesFrusta,
            ColorGrading,
            ComputedVisibility,
            CubemapFrusta,
            Frustum,
            OrthographicProjection,
            PerspectiveProjection,
            Projection,
            RenderLayers,
            SkinnedMesh,
            Visibility,
            VisibleEntities,
        ];
    }

    #[cfg(feature = "register_ui")]
    {
        use bevy::ui::{
            prelude::{Button, CalculatedClip, CalculatedSize, Label, Node, Style, UiImage},
            BackgroundColor, FocusPolicy, Interaction, RelativeCursorPosition, ZIndex,
        };
        register_reflect_query![
            app,
            BackgroundColor,
            Button,
            // CalculatedClip,
            CalculatedSize,
            FocusPolicy,
            Interaction,
            Label,
            Node,
            // RelativeCursorPosition,
            Style,
            UiImage,
            // ZIndex
        ];
    }

    #[cfg(feature = "register_text")]
    {
        use bevy::text::{Text, Text2dBounds};
        register_reflect_query![app, /*Text2dBounds,*/ Text];
    }
}
