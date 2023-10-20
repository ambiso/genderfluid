@group(0) @binding(0)
var texture: texture_storage_2d<rgba8unorm, read_write>;

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

    textureStore(texture, location, color);
}

fn get_data(location: vec2<i32>, offset_x: i32, offset_y: i32) -> vec4<f32> {
    let value: vec4<f32> = textureLoad(texture, location + vec2<i32>(offset_x, offset_y));
    return value;
}

fn get_height(data: vec4<f32>) -> f32 {
    return data.x;
}

fn get_vel(data: vec4<f32>) -> f32 {
    return data.y;
}

fn pack_data(height: f32, vel: f32) -> vec4<f32> {
    return vec4(height, vel, 0.0, 0.0);
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let d0 = get_data(location,  0,  0);
    let d1 = get_data(location,  1,  0);
    let d2 = get_data(location, -1,  0);
    let d3 = get_data(location,  0,  1);
    let d4 = get_data(location,  0, -1);

    let dt = 0.01 * 0.25;

    let accel = get_height(d1) + get_height(d2) + get_height(d3) + get_height(d4) - 4.0 * get_height(d0);
    let newVel = get_vel(d0) + accel * dt;
    let newHeight = get_height(d0) + newVel * dt;

    storageBarrier();

    textureStore(texture, location, pack_data(newHeight, newVel));
}
