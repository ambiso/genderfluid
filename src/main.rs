//! A compute shader that simulates Genderfluid.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

mod extract_heights;
mod orbit_camera;
mod water_pbr_material;
use bevy::{
    core::{Pod, Zeroable},
    gltf::Gltf,
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::NoFrustumCulling,
        Render, RenderApp, RenderSet,
    },
    window::WindowPlugin,
};
use bevy_shader_utils::ShaderUtilsPlugin;
use extract_heights::{
    GenderfluidExtractNode, GenderfluidExtractPipeline, GenderfluidImage, QueryPosition,
};
use orbit_camera::{ControlEvent, OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin};
use smooth_bevy_cameras::LookTransformPlugin;
use std::{borrow::Cow, f32::consts::PI};
use water_pbr_material::WaterStandardMaterial;

const SIZE: u32 = 256;
const WORKGROUP_SIZE: u32 = 8;
const CELL_SIZE: u32 = 32;

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

#[derive(Default, Component)]
pub struct Plant {
    pub health: f32,
}
#[derive(Resource)]
pub struct PlantGrid {
    pub grid: Vec<Vec<Option<Entity>>>,
}

#[derive(Resource)]
struct PlantAsset(Handle<Scene>);

impl Default for PlantGrid {
    fn default() -> Self {
        Self {
            grid: vec![
                vec![None; SIZE as usize / CELL_SIZE as usize];
                SIZE as usize / CELL_SIZE as usize
            ],
        }
    }
}

#[derive(Default, Component)]
pub struct SphereController {
    pub enabled: bool,
    pub translate_sensitivity: f32,
}

#[derive(Event)]
pub enum SphereControlEvent {
    Translate(Vec3),
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
            OrbitCameraPlugin::default(),
            LookTransformPlugin,
            ShaderUtilsPlugin,
            MaterialPlugin::<WaterStandardMaterial>::default(),
        ))
        .add_event::<SphereControlEvent>()
        .insert_resource(PlantGrid::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                sphere_input_map,
                move_sphere,
                cursor_grab_system,
                prepare_fluid_compute_uniforms,
                spawn_plants,
                update_camera_target,
                update_plant_health,
            ),
        )
        .run();
}

use bevy::window::CursorGrabMode;

fn cursor_grab_system(
    mut windows: Query<&mut Window>,
    btn: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
) {
    let mut window = windows.single_mut();

    if btn.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }

    if key.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
}

fn update_camera_target(
    mut events: EventWriter<ControlEvent>,
    player: Query<&Transform, With<Player>>,
) {
    events.send(ControlEvent::NewTarget(player.single().translation));
}

#[derive(Resource, Reflect, Debug, Clone, TypeUuid, ShaderType, Pod, Zeroable, Copy)]
#[repr(C)]
#[uuid = "61e3fe7d-e307-4d7f-a060-35fff2cba963"]
struct FluidComputeUniforms {
    player_position: Vec2,
    click: u32,
    _padding: u32,
}

fn spawn_plants(
    mut commands: Commands,
    time: Res<Time>,
    mut plant_grid: ResMut<PlantGrid>,
    plant_asset: Res<PlantAsset>,
) {
    // Some condition to control when to spawn a new plant, e.g., every second
    static mut LAST_SPAWN_TIME: f64 = 0.0;
    let current_time = time.elapsed_seconds_f64();
    if current_time - unsafe { LAST_SPAWN_TIME } < 0.015 {
        return;
    }
    unsafe {
        LAST_SPAWN_TIME = current_time;
    }

    let mut rng = rand::thread_rng();
    use rand::Rng;

    // Loop through the grid to find an empty spot to place a new plant
    for i in 0..(SIZE / CELL_SIZE) {
        for j in 0..(SIZE / CELL_SIZE) {
            if plant_grid.grid[i as usize][j as usize].is_none() {
                let offset_x: f32 = rng.gen_range(0.0..=CELL_SIZE as f32);
                let offset_z: f32 = rng.gen_range(0.0..=CELL_SIZE as f32);

                let world_x = i as f32 * CELL_SIZE as f32 + offset_x - SIZE as f32 / 2.0;
                let world_z = j as f32 * CELL_SIZE as f32 + offset_z - SIZE as f32 / 2.0;

                // Spawn a new plant entity
                let new_plant = commands
                    .spawn(SceneBundle {
                        scene: plant_asset.0.clone(),
                        transform: Transform::from_xyz(world_x * 0.02, 2.0, world_z * 0.02)
                            .with_scale(Vec3::splat(0.02)),
                        ..Default::default()
                    })
                    .insert(Plant::default())
                    .id();

                // Update the grid
                plant_grid.grid[i as usize][j as usize] = Some(new_plant);

                // Possibly break if you only want to spawn one plant per update
                return;
            }
        }
    }
}

fn update_plant_health(
    time: Res<Time>,
    mut plant_grid: ResMut<PlantGrid>,
    mut plants: Query<(&mut Transform, &Plant)>,
) {
    let current_time = time.elapsed_seconds() as f32;

    for i in 0..(SIZE / CELL_SIZE) {
        for j in 0..(SIZE / CELL_SIZE) {
            if let Some(plant) = plant_grid.grid[i as usize][j as usize] {
                // Example logic to update health based on sin(time)
                // plant.health = 50.0 * (current_time * PI).sin() + 50.0; // Range from 0 to 100

                // Update scale based on health. This is just an example,
                // you can define your own logic to map health to scale.
                // let scale = 0.1 + (plant.health / 1000.0);
                // transform.scale = Vec3::splat(scale);

                if let Ok((mut transform, actual_plant)) = plants.get_mut(plant) {
                    transform.scale = Vec3::splat(0.01 * (current_time * PI).sin() + 0.01);
                }
                // println!("Updated plant size");
            }
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut custom_materials: ResMut<Assets<WaterStandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    let eye = Vec3::new(-2.0, 5.0, 5.1);
    let target = Vec3::default();
    let controllllller = OrbitCameraController::default();

    let plant = asset_server.load("glowingflower2.glb#Scene0");
    commands.insert_resource(PlantAsset(plant));
    // to position our 3d model, simply use the Transform
    // in the SceneBundle
    // commands
    //     .spawn(SceneBundle {
    //         scene: plant,
    //         transform: Transform::from_xyz(0.0, 2.0, -5.0).with_scale(Vec3::splat(0.1)),
    //         ..Default::default()
    //     })
    //     .insert(Plant::default());

    commands
        .spawn(Camera3dBundle::default())
        .insert(OrbitCameraBundle::new(controllllller, eye, target, Vec3::Y));

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
    let entity_spawn = Vec3::new(0.0, 3.0, 0.0);
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1337,
                    sectors: 32,
                    stacks: 32,
                })),
                material: standard_materials.add(Color::WHITE.into()),
                transform: Transform::from_translation(entity_spawn),
                ..default()
            },
            Movable::new(entity_spawn),
        ))
        .insert(SphereController {
            enabled: true,
            translate_sensitivity: 2.0,
            ..Default::default()
        })
        .insert(Player);

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
    let terrain_height = make_texture();

    let material_handle = custom_materials.add(WaterStandardMaterial {
        height: Some(height1.clone()),
        velocity: Some(velocity.clone()),
        terrain: Some(terrain_height.clone()),
        base_color: Color::hsla(200.0, 1.0, 0.5, 0.8),
        alpha_mode: AlphaMode::Blend,
        // metallic: 0.5,
        reflectance: 1.0,
        is_water: 1,
        ..Default::default()
    });

    commands
        .spawn(MaterialMeshBundle {
            mesh: meshes.add(
                shape::Plane {
                    size: 5.0,
                    subdivisions: SIZE,
                }
                .into(),
            ),
            material: material_handle,
            ..default()
        })
        .insert(NoFrustumCulling);

    let terrain_material_handle = custom_materials.add(WaterStandardMaterial {
        height: Some(terrain_height.clone()),
        base_color: Color::hsla(22.0, 0.6, 0.28, 1.0),
        alpha_mode: AlphaMode::Opaque,
        // metallic: 0.5,
        reflectance: 0.2,
        is_water: 0,
        ..Default::default()
    });

    commands
        .spawn(MaterialMeshBundle {
            mesh: meshes.add(
                shape::Plane {
                    size: 5.0,
                    subdivisions: SIZE,
                }
                .into(),
            ),
            material: terrain_material_handle,
            ..default()
        })
        .insert(NoFrustumCulling);

    let water_compute_uniforms_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid compute uniforms"),
        size: std::mem::size_of::<FluidComputeUniforms>() as u64,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let extract_positions = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid extract positions"),
        size: std::mem::size_of::<QueryPosition>() as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::MAP_WRITE,
        mapped_at_creation: false,
    });

    let extract_height = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid extract height"),
        size: std::mem::size_of::<f32>() as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let extract_terrain_height = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid extract terrain height"),
        size: std::mem::size_of::<f32>() as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    commands.insert_resource(GenderfluidImage {
        height1,
        height2,
        velocity,
        terrain_height,
        uniforms: water_compute_uniforms_buffer,
        extract_positions,
        extract_height,
        extract_terrain_height,
    });
}

// This system will move all Movable entities with a Transform
pub fn move_sphere(
    mut spheres: Query<(&mut Transform, &mut SphereController)>,
    mut events: EventReader<SphereControlEvent>,
    timer: Res<Time>,
) {
    for (mut transform, controller) in &mut spheres {
        if !controller.enabled {
            continue;
        }

        for event in events.iter() {
            match event {
                SphereControlEvent::Translate(dir) => {
                    let movement = *dir * timer.delta_seconds();
                    // println!("moving: {}", movement);
                    transform.translation += movement;
                    println!("transform: {}", transform.translation);
                }
            }
        }
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
        render_app.add_systems(
            Render,
            extract_heights::queue_extract_bind_group.in_set(RenderSet::Queue),
        );

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("genderfluid", GenderfluidNode::default());
        render_graph.add_node("genderfluid extract", GenderfluidExtractNode::default());
        render_graph.add_node_edge("genderfluid", bevy::render::main_graph::node::CAMERA_DRIVER);
        render_graph.add_node_edge(
            "genderfluid extract",
            bevy::render::main_graph::node::CAMERA_DRIVER,
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GenderfluidPipeline>();
        render_app.init_resource::<GenderfluidExtractPipeline>();
    }
}

#[derive(Component)]
pub struct Player;

fn prepare_fluid_compute_uniforms(
    btn: Res<Input<MouseButton>>,
    render_queue: Res<RenderQueue>,
    genderfluidimage: ResMut<GenderfluidImage>,
    player: Query<&Transform, With<Player>>,
    // plants: Query<&Transform, With<Plant>>,
) {
    // write `time.seconds_since_startup` as a `&[u8]`
    // into the time buffer at offset 0.
    render_queue.write_buffer(
        &genderfluidimage.uniforms,
        0,
        bevy::core::bytes_of(&FluidComputeUniforms {
            player_position: {
                let t = player.single().translation;
                Vec2::new(t.x, t.z) / 5.0 + 0.5
            },
            click: btn.pressed(MouseButton::Left) as u32,
            _padding: 0,
        }),
    );
    // render_queue.write_buffer(
    //     &genderfluidimage.uniforms,
    //     0,
    //     bevy::core::cast_slice(&),
    // );
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
    let terrain_height = &gpu_images[&genderfluid_image.terrain_height];

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
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(&terrain_height.texture_view),
            },
            BindGroupEntry {
                binding: 4,
                resource: genderfluid_image.uniforms.as_entire_binding(),
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
                        // height_in
                        make_binding(0, StorageTextureAccess::ReadOnly),
                        // height_out
                        make_binding(1, StorageTextureAccess::WriteOnly),
                        // velocity
                        make_binding(2, StorageTextureAccess::ReadWrite),
                        // terrain_height_in
                        make_binding(3, StorageTextureAccess::ReadWrite),
                        // uniforms
                        BindGroupLayoutEntry {
                            binding: 4,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(std::mem::size_of::<
                                    FluidComputeUniforms,
                                >(
                                )
                                    as u64),
                            },
                            count: None,
                        },
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

// Add this system to handle sphere input
pub fn sphere_input_map(
    mut events: EventWriter<SphereControlEvent>,
    keyboard: Res<Input<KeyCode>>,
    controllers: Query<&SphereController>,
    player: Query<&Transform, With<Player>>,
    camera: Query<&Transform, With<OrbitCameraController>>,
) {
    // Can only control one sphere at a time.
    let controller = if let Some(controller) = controllers.iter().find(|c| c.enabled) {
        controller
    } else {
        return;
    };

    let SphereController {
        translate_sensitivity,
        ..
    } = *controller;

    let mut view_direction = player.single().translation - camera.single().translation;
    view_direction.y = 0.0;
    view_direction = view_direction.normalize();

    let left = Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, PI / 2.0, 0.0))
        * view_direction;

    for (key, dir) in [
        (KeyCode::W, view_direction),
        (KeyCode::A, left),
        (KeyCode::S, -view_direction),
        (KeyCode::D, -left),
    ]
    .iter()
    .cloned()
    {
        if keyboard.pressed(key) {
            events.send(SphereControlEvent::Translate(translate_sensitivity * dir));
        }
    }
}
