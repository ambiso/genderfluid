#import bevy_shader_utils::simplex_noise_3d simplex_noise_3d

@group(0) @binding(0)
var height_in: texture_storage_2d<r32float, read>;
@group(0) @binding(1)
var height_out: texture_storage_2d<r32float, write>;
@group(0) @binding(2)
var velocity: texture_storage_2d<r32float, read_write>;
@group(0) @binding(3)
var terrain_height_in: texture_storage_2d<r32float, read_write>;

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let location_for_noise = vec3<f32>(f32(invocation_id.x) * 0.052, f32(invocation_id.y) * 0.052, 1.0);
    let noise = simplex_noise_3d(location_for_noise);
    let height = noise + 1.5;

    textureStore(height_out, location, vec4<f32>(max(height, 0.0), 0.0, 0.0, 1.0));
    textureStore(velocity, location, vec4(0.0, 0.0, 0.0, 1.0));

    let location_for_noise_for_terrain = vec3<f32>(f32(invocation_id.x) * 10.052, f32(invocation_id.y) * 0.152, 0.0);
    let noise_for_terrain = simplex_noise_3d(location_for_noise);
    let height_for_terrain = noise_for_terrain + 0.1;
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

fn get_vel(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(velocity, location + vec2<i32>(offset_x, offset_y));
    return value.x;
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

    let dt = 1.0 / 60.0;
    let damping = 0.9971349;
    let k = 10.5820;

    // Calculate the total height difference from neighbors
    let accel = k * (height1 + height2 + height3 + height4 - 4.0 * height0);

    // Update velocity with damping
    let new_vel = get_vel(location, 0, 0) * damping + accel * dt;

    // Mass conservation: distribute the change in height back to neighbors
    let height_change = new_vel * dt;
    let new_height = height0 + height_change;

    textureStore(velocity, location, vec4(new_vel, 0.0, 0.0, 1.0));
    textureStore(height_out, location, vec4(max(new_height, 0.0), 0.0, 0.0, 1.0));
}

