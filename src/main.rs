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
use smooth_bevy_cameras::{
    controllers::fps::{FpsCameraBundle, FpsCameraController, FpsCameraPlugin},
    LookTransform, LookTransformBundle, LookTransformPlugin, Smoother,
};
use std::borrow::Cow;

const SIZE: u32 = 256;
const WORKGROUP_SIZE: u32 = 8;

// Define a struct to keep some information about our entity.
// Here it's an arbitrary movement speed, the spawn location, and a maximum distance from it.
#[derive(Component)]
struct Movable {
    spawn: Vec3,
    max_distance: f32,
    speed: f32,
}

// Implement a utility function for easier Movable struct creation.
impl Movable {
    fn new(spawn: Vec3) -> Self {
        Movable {
            spawn,
            max_distance: 5.0,
            speed: 2.0,
        }
    }
}

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
            FpsCameraPlugin::default(),
            LookTransformPlugin,
            MaterialPlugin::<CustomMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, move_cube)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut custom_materials: ResMut<Assets<CustomMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let eye = Vec3::new(-2.0, 5.0, 5.1);
    let target = Vec3::default();
    let controllllller = FpsCameraController::default();

    commands
        .spawn(Camera3dBundle::default())
        .insert(FpsCameraBundle::new(controllllller, eye, target, Vec3::Y));
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
    // Add a cube to visualize translation.
    let entity_spawn = Vec3::ZERO;
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 1.0,
                sectors: 8,
                stacks: 8,
            })),
            material: standard_materials.add(Color::WHITE.into()),
            transform: Transform::from_translation(entity_spawn),
            ..default()
        },
        Movable::new(entity_spawn),
    ));

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

// This system will move all Movable entities with a Transform
fn move_cube(mut cubes: Query<(&mut Transform, &mut Movable)>, timer: Res<Time>) {
    for (mut transform, mut cube) in &mut cubes {
        // Check if the entity moved too far from its spawn, if so invert the moving direction.
        if (cube.spawn - transform.translation).length() > cube.max_distance {
            cube.speed *= -1.0;
        }
        let direction = transform.local_x();
        transform.translation += direction * cube.speed * timer.delta_seconds();
    }
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
        let make_binding = |binding: u32, access: StorageTextureAccess| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access,
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
                    entries: &[
                        make_binding(0, StorageTextureAccess::ReadOnly),
                        make_binding(1, StorageTextureAccess::WriteOnly),
                        make_binding(2, StorageTextureAccess::ReadWrite),
                    ],
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
