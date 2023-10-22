#import bevy_shader_utils::simplex_noise_3d simplex_noise_3d

struct unsereigenerty {
    player_position: vec2<f32>,
    click: u32,
}

@group(0) @binding(0)
var height_in: texture_storage_2d<r32float, read>;
@group(0) @binding(1)
var height_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2)
var velocity: texture_storage_2d<r32float, read_write>;
@group(0) @binding(3)
var terrain_height_in: texture_storage_2d<r32float, read_write>;
@group(0) @binding(4)
var<uniform> uniforms : unsereigenerty;

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let location_for_noise = vec3<f32>(f32(invocation_id.x) * 0.0052, f32(invocation_id.y) * 0.0052, 1.0);
    let noise = simplex_noise_3d(location_for_noise);
    var height = noise * 1.0 - 0.777;
    if (location.y < 100) {
        height = 0.0;
    }

    textureStore(height_out, location, vec4<f32>(max(height, 0.0), 0.0, 0.0, 1.0));
    textureStore(velocity, location, vec4(0.0, 0.0, 0.0, 1.0));

    let location_for_noise_for_terrain = vec3<f32>(f32(invocation_id.x) * 0.0052, f32(invocation_id.y) * 0.0152, 0.0);
    let noise_for_terrain = simplex_noise_3d(location_for_noise_for_terrain);
    let height_for_terrain = noise_for_terrain + 1.5;
    textureStore(terrain_height_in, location, vec4<f32>(max(height_for_terrain, 0.0), 0.0, 0.0, 1.0));
}

fn get_height(location: vec2<i32>, offset_x: i32, offset_y: i32, center_height: f32, dim: vec2<u32>) -> f32 {
    let loc = location + vec2<i32>(offset_x, offset_y);
    if (loc.x < 0 || loc.y < 0 || loc.x >= i32(dim.x) || loc.y >= i32(dim.y)) {
        return center_height;
    }
    let value: vec4<f32> = textureLoad(height_in, loc);
    return value.x;
}

fn get_terrain_height(location: vec2<i32>, offset_x: i32, offset_y: i32, center_height: f32, dim: vec2<u32>) -> f32 {
    let loc = location + vec2<i32>(offset_x, offset_y);
    if (loc.x < 0 || loc.y < 0 || loc.x >= i32(dim.x) || loc.y >= i32(dim.y)) {
        return center_height;
    }
    let value: vec4<f32> = textureLoad(terrain_height_in, loc);
    return value.x;
}

fn get_vel(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(velocity, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn cell_flow(w0: f32, h0: f32, w1: f32, h1: f32) -> f32
{
    var diff = (w1 + h1) - (w0 + h0);
    // var diff = (w1) - (w0);
    var drop = 0.0;

    if ((w1 < 0.001 && h1 > w0 + h0) || (w0 < 0.001 && h0 > w1 + h1) || abs(diff) < 0.001) {
        return (drop);
    }
    return diff;
    // if (w0 + h0 > w1 + h1)
    // {
    //     return (-diff );
    // }
    // if (w0 + h0 < w1 + h1)
    // {
    //     return  (diff);
    // }
    // return (0.0);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let height0 = textureLoad(height_in, location).x;
    let dim = textureDimensions(height_in);
    let height1 = get_height(location,  1,  0, height0, dim);
    let height2 = get_height(location, -1,  0, height0, dim);
    let height3 = get_height(location,  0,  1, height0, dim);
    let height4 = get_height(location,  0, -1, height0, dim);

    let terrain_height0 = textureLoad(terrain_height_in, location).x;
    let terrain_dim = textureDimensions(terrain_height_in);
    let terrain_height1 = get_terrain_height(location,  1,  0, terrain_height0, terrain_dim);
    let terrain_height2 = get_terrain_height(location, -1,  0, terrain_height0, terrain_dim);
    let terrain_height3 = get_terrain_height(location,  0,  1, terrain_height0, terrain_dim);
    let terrain_height4 = get_terrain_height(location,  0, -1, terrain_height0, terrain_dim);

    let dt = 1.0 / 60.0;
    let damping = 0.9971349;
    let k = 50.15820;

    let attracking_point = uniforms.player_position;
    let uv = vec2(f32(location.x) / f32(dim.x), f32(location.y) / f32(dim.y));
    let v: vec2<f32> = attracking_point - uv;
    var attracting_force = 0.0;
    if (uniforms.click == u32(1) && length(v) < 0.04) {
        attracting_force = min(20.0, 1.0/((abs(v.x * v.x * v.x) + abs(v.y * v.y * v.y)))) / 10000.0;
    }

    // Calculate the total height difference from neighbors
    // let accel = k * ((height1) + (height2) + (height3) + (height4) - 4.0 * (height0));
    let accel = k * (cell_flow(height0, terrain_height0, height1, terrain_height1)
                    + cell_flow(height0, terrain_height0, height2, terrain_height2)
                    + cell_flow(height0, terrain_height0, height3, terrain_height3)
                    + cell_flow(height0, terrain_height0, height4, terrain_height4) + attracting_force);
	
    // Update velocity with damping
    let new_vel = get_vel(location, 0, 0) * damping + accel * dt;

    // Mass conservation: distribute the change in height back to neighbors
    let height_change = new_vel * dt;
    var new_height = height0 + height_change;
	if (terrain_height0 < 0.777) {
		new_height -= 0.012;
	}
	new_height *= 0.99999;
    textureStore(velocity, location, vec4(new_vel, 0.0, 0.0, 1.0));
    textureStore(height_out, location, vec4(max(new_height, 0.0), 0.0, 0.0, 1.0));
}

