use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_resource::ExtractResource,
        render_asset::RenderAssets,
        render_graph::{self},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use bytemuck::{Pod, Zeroable};
use std::{borrow::Cow, num::NonZeroU32};

use crate::{SIZE, WORKGROUP_SIZE};

#[derive(Resource, Clone, ExtractResource)]
pub struct GenderfluidImage {
    pub height1: Handle<Image>,
    pub height2: Handle<Image>,
    pub velocity: Handle<Image>,
    pub terrain_height: Handle<Image>,
    pub uniforms: Buffer,
	pub extract_positions: Buffer,
	pub extract_height: Buffer,
	pub extract_terrain_height: Buffer,
}

#[derive(Resource)]
pub struct GenderfluidExtractImageBindGroup(pub BindGroup);

pub fn queue_extract_bind_group(
    mut commands: Commands,
    pipeline: Res<GenderfluidExtractPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    genderfluid_image: Res<GenderfluidImage>,
    render_device: Res<RenderDevice>,
) {
    let height1 = &gpu_images[&genderfluid_image.height1];
    // let height2 = &gpu_images[&genderfluid_image.height2];
    // let velocity = &gpu_images[&genderfluid_image.velocity];
    let terrain_height = &gpu_images[&genderfluid_image.terrain_height];

    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.texture_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&height1.texture_view),
            },
            // BindGroupEntry {
            //     binding: 1,
            //     resource: BindingResource::TextureView(&height2.texture_view),
            // },
            // BindGroupEntry {
            //     binding: 2,
            //     resource: BindingResource::TextureView(&velocity.texture_view),
            // },
            BindGroupEntry {
                binding: 3,
                resource: BindingResource::TextureView(&terrain_height.texture_view),
            },
            // BindGroupEntry {
            //     binding: 4,
            //     resource: genderfluid_image.uniforms.as_entire_binding(),
            // },
            BindGroupEntry {
                binding: 5,
                resource: genderfluid_image.extract_positions.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 6,
                resource: genderfluid_image.extract_height.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 7,
                resource: genderfluid_image.extract_terrain_height.as_entire_binding(),
            },
        ],
    });
    commands.insert_resource(GenderfluidExtractImageBindGroup(bind_group));
}

#[derive(Resource)]
pub struct GenderfluidExtractPipeline {
    texture_bind_group_layout: BindGroupLayout,
    update_pipeline: CachedComputePipelineId,
}

#[derive(Resource, Reflect, Debug, Clone, TypeUuid, ShaderType, Pod, Zeroable, Copy)]
#[repr(C)]
#[uuid = "657741ad-e8f8-43dc-bf2b-9b79c43e38e9"]
pub struct QueryPosition {
    pub x: u32,
	pub y: u32,
}

impl FromWorld for GenderfluidExtractPipeline {
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
        let make_extract_binding = |binding: u32, ro: bool, size: usize| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: ro },
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(size as u64),
            },
            count: Some(NonZeroU32::new((SIZE / 4) * (SIZE / 4)).unwrap()),
        };
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        // height_in
                        make_binding(0, StorageTextureAccess::ReadOnly),
                        // velocity
                        // make_binding(2, StorageTextureAccess::ReadOnly),
                        // terrain_height_in
                        make_binding(3, StorageTextureAccess::ReadOnly),
                        // extract_positions
                        make_extract_binding(5, true, std::mem::size_of::<QueryPosition>()),
                        // extract_height_out
                        make_extract_binding(6, false, std::mem::size_of::<f32>()),
                        // extract_terrain_height_out
                        make_extract_binding(7, false, std::mem::size_of::<f32>()),
                    ],
                });
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/height_map_extract_compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("extract"),
        });

        GenderfluidExtractPipeline {
            texture_bind_group_layout,
            update_pipeline,
        }
    }
}

enum GenderfluidState {
    Loading,
    Update,
}

pub struct GenderfluidExtractNode {
    state: GenderfluidState,
}

impl Default for GenderfluidExtractNode {
    fn default() -> Self {
        Self {
            state: GenderfluidState::Loading,
        }
    }
}

impl render_graph::Node for GenderfluidExtractNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GenderfluidExtractPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            GenderfluidState::Loading => {
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
        let texture_bind_group = &world.resource::<GenderfluidExtractImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GenderfluidExtractPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            GenderfluidState::Loading => {}
            GenderfluidState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE / WORKGROUP_SIZE, 1, 1);
            }
        }

        Ok(())
    }
}
