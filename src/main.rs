//! A compute shader that simulates Genderfluid.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayout},
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
            VertexFormat,
        },
    },
    window::WindowPlugin,
};
use std::borrow::Cow;

const SIZE: u32 = 256;
const WORKGROUP_SIZE: u32 = 8;

// const ATTRIBUTE_BLEND_COLOR: MeshVertexAttribute =
//     MeshVertexAttribute::new("BlendColor", 988540917, VertexFormat::Float32x4);

// A simple 3D scene with light shining over a cube sitting on a plane.
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // uncomment for unthrottled FPS
                    // present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            GenderfluidComputePlugin,
            MaterialPlugin::<CustomMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    //mut materials: ResMut<Assets<StandardMaterial>>,
    mut custom_materials: ResMut<Assets<CustomMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let mut make_texture = || {
        let mut texture = Image::new_fill(
            Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::R32Float,
        );
        texture.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;
        let image = images.add(texture);
        image
    };
    let height1 = make_texture();
    let height2 = make_texture();
    let velocity = make_texture();

    // commands.spawn(SpriteBundle {
    //     sprite: Sprite {
    //         custom_size: Some(Vec2::new(SIZE as f32, SIZE as f32)),
    //         ..default()
    //     },
    //     texture: image.clone(),
    //     ..default()
    // });
    // commands.spawn(Camera2dBundle::default());

    // plane
    // let material_handle = materials.add(StandardMaterial {
    //     base_color_texture: Some(image.clone()),
    //     alpha_mode: AlphaMode::Blend,
    //     unlit: true,
    //     ..default()
    // });
    let material_handle = custom_materials.add(CustomMaterial {
        color: Color::WHITE,
        size: SIZE,
        height: Some(height1.clone()), // TODO richtiges ding reinpassen
        velocity: Some(velocity.clone()),
    });

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(
            shape::Plane {
                size: 5.0,
                subdivisions: SIZE,
            }
            .into(),
        ),
        material: material_handle,
        ..default()
    });

    commands.insert_resource(GenderfluidImage {
        height1,
        height2,
        velocity,
    });
}

pub struct GenderfluidComputePlugin;

impl Plugin for GenderfluidComputePlugin {
    fn build(&self, app: &mut App) {
        // Extract the genderfluid image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugins(ExtractResourcePlugin::<GenderfluidImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, queue_bind_group.in_set(RenderSet::Queue));

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("genderfluid", GenderfluidNode::default());
        render_graph.add_node_edge("genderfluid", bevy::render::main_graph::node::CAMERA_DRIVER);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GenderfluidPipeline>();
    }
}

#[derive(Resource, Clone, ExtractResource)]
struct GenderfluidImage {
    height1: Handle<Image>,
    height2: Handle<Image>,
    velocity: Handle<Image>,
}

#[derive(Resource)]
struct GenderfluidImageBindGroup(BindGroup);

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<GenderfluidPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    mut genderfluid_image: ResMut<GenderfluidImage>,
    render_device: Res<RenderDevice>,
) {
    let gfi = &mut *genderfluid_image;
    std::mem::swap(&mut gfi.height1, &mut gfi.height2);

    let height1 = &gpu_images[&genderfluid_image.height1];
    let height2 = &gpu_images[&genderfluid_image.height2];
    let velocity = &gpu_images[&genderfluid_image.velocity];

    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.texture_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&height1.texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&height2.texture_view),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&velocity.texture_view),
            },
        ],
    });
    commands.insert_resource(GenderfluidImageBindGroup(bind_group));
}

#[derive(Resource)]
pub struct GenderfluidPipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for GenderfluidPipeline {
    fn from_world(world: &mut World) -> Self {
        let make_binding = |binding: u32| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::ReadWrite,
                format: TextureFormat::R32Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        };
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[make_binding(0), make_binding(1), make_binding(2)],
                });
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/fluid_heightmap_berechnungsschattierer.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        GenderfluidPipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum GenderfluidState {
    Loading,
    Init,
    Update,
}

struct GenderfluidNode {
    state: GenderfluidState,
}

impl Default for GenderfluidNode {
    fn default() -> Self {
        Self {
            state: GenderfluidState::Loading,
        }
    }
}

impl render_graph::Node for GenderfluidNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GenderfluidPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            GenderfluidState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = GenderfluidState::Init;
                }
            }
            GenderfluidState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = GenderfluidState::Update;
                }
            }
            GenderfluidState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<GenderfluidImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GenderfluidPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            GenderfluidState::Loading => {}
            GenderfluidState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, 1);
            }
            GenderfluidState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, Debug, Clone, TypeUuid, TypePath)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct CustomMaterial {
    #[uniform(0)]
    color: Color,
    #[uniform(0)]
    size: u32,
    #[texture(1)]
    #[sampler(2)]
    height: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    velocity: Option<Handle<Image>>,
}

impl Material for CustomMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/water_vertex_and_fragment.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/water_vertex_and_fragment.wgsl".into()
    }
}
