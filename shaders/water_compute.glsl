#[compute]
#version 450

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(set = 0, binding = 0, std430) restrict buffer HeightDataBuffer {
    float heightmap[];
} height_data_buffer;

layout(set = 0, binding = 1, std430) restrict buffer VelocityDataBuffer {
    vec2 velocity[];
} velocity_data_buffer;

float get_height(int x, int y, int width) {
    return height_data_buffer.heightmap[y * width + x];
}

void main() {
    uvec3 gid = gl_GlobalInvocationID;
    // int width = int(gl_WorkGroupSize.x * gl_NumWorkGroups.x);
    // int height = int(gl_WorkGroupSize.y * gl_NumWorkGroups.y);
    int width = 3;
    int height = 3;

    if(gid.x >= width || gid.y >= height) return;

    int x = int(gid.x);
    int y = int(gid.y);
    
    // Neighbor heights
    float hLeft = get_height(x - 1, y, width);
    float hRight = get_height(x + 1, y, width);
    float hUp = get_height(x, y - 1, width);
    float hDown = get_height(x, y + 1, width);
    
    // Current height and velocity
    float hCurrent = get_height(x, y, width);
    vec2 velocity = velocity_data_buffer.velocity[y * width + x];

    // Update logic here
    // ...

    // For example, update height based on some factor of neighboring heights and current velocity
    float newHeight = hCurrent + 0.1 * (hLeft + hRight + hUp + hDown - 4.0 * hCurrent);

    // Update height
    height_data_buffer.heightmap[y * width + x] = newHeight;

    // Similarly, you can update the velocity_data_buffer as per your simulation needs
}
