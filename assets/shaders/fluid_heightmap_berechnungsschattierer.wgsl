@group(0) @binding(0)
var height_in: texture_storage_2d<r32float, read_write>;
@group(0) @binding(1)
var height_out: texture_storage_2d<r32float, read_write>;
@group(0) @binding(2)
var velocity: texture_storage_2d<r32float, read_write>;

fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    return state;
}

fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let randomNumber = randomFloat(invocation_id.y * num_workgroups.x + invocation_id.x);
    let alive = randomNumber > 0.9;
    let color = vec4<f32>(f32(alive));

    textureStore(height_out, location, color);
}

fn get_height(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(height_in, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn get_vel(location: vec2<i32>, offset_x: i32, offset_y: i32) -> f32 {
    let value: vec4<f32> = textureLoad(velocity, location + vec2<i32>(offset_x, offset_y));
    return value.x;
}

fn pack_data(height: f32, vel: f32) -> vec4<f32> {
    return vec4(height, vel, 0.0, 0.0);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let height0 = get_height(location,  0,  0);
    let height1 = get_height(location,  1,  0);
    let height2 = get_height(location, -1,  0);
    let height3 = get_height(location,  0,  1);
    let height4 = get_height(location,  0, -1);

    let dt = 0.01;

    let k = 0.1;
    let accel = k * (height1 + height2 + height3 + height4 - 4.0 * height0);
    let new_vel = get_vel(location, 0, 0) + accel * dt;
    textureStore(velocity, location, vec4(new_vel, 0.0, 0.0, 1.0));

    let new_height = height0 + new_vel * dt;
    textureStore(height_out, location, vec4(new_height, 0.0, 0.0, 1.0));
}
