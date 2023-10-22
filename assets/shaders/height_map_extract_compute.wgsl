// struct unsereigenerty {
//     player_position: vec2<f32>,
//     click: u32,
// }

@group(0) @binding(0)
var height_in: texture_storage_2d<r32float, read>;
// @group(0) @binding(1)
// var height_out: texture_storage_2d<r32float, write>;
// @group(0) @binding(2)
// var velocity: texture_storage_2d<r32float, read_write>;
@group(0) @binding(3)
var terrain_height_in: texture_storage_2d<r32float, read>;
// @group(0) @binding(4)
// var<uniform> uniforms : unsereigenerty;

@group(0) @binding(5)
var<storage, read> extract_position: array<vec2<i32>>;
@group(0) @binding(6)
var<storage, read_write> extract_height: array<f32>;
@group(0) @binding(7)
var<storage, read_write> extract_terrain_height: array<f32>;

@compute @workgroup_size(8, 8, 1)
fn extract(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
	extract_height[invocation_id.x] = textureLoad(height_in, extract_position[invocation_id.x]).x;
	extract_terrain_height[invocation_id.x] = textureLoad(terrain_height_in, extract_position[invocation_id.x]).x;
}


