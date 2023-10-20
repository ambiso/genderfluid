// #import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_bindings   mesh
#import bevy_pbr::mesh_functions  mesh_position_local_to_clip

struct CustomMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: CustomMaterial;
@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(1) @binding(2)
var base_color_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
	// var height_offset: vec4<f32> = textureSample(base_color_texture, base_color_sampler, vertex.position.xz);
	var height_offset: vec4<f32> = textureLoad(base_color_texture, vec2<i32>(i32(vertex.position.x), i32(vertex.position.z)), 0);
    out.clip_position = mesh_position_local_to_clip(
        mesh.model,
        vec4<f32>(vertex.position + vec3(0.0, height_offset.x, 0.0) * 10.0, 1.0)
    );
	out.uv = vertex.position.xz / 5.0 + 0.5;
    return out;
}

@fragment
fn fragment(
	input: VertexOutput,
) -> @location(0) vec4<f32> {
	var height_offset: vec4<f32> = textureSample(base_color_texture, base_color_sampler, input.uv);
    return height_offset;
}