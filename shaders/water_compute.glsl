#[compute]
#version 450

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(set = 0, binding = 0, std430) restrict buffer HeightInputBuffer {
    float heightmap[];
} height_input_buffer;

layout(set = 0, binding = 1, std430) restrict buffer HeightOutputBuffer {
    float heightmap[];
} height_output_buffer;

layout(set = 0, binding = 2, std430) restrict buffer VelocityDataBuffer {
    float velocity[];
} velocity_buffer;

float get_height(int x, int y, int size) {
    if (x < 0 || x >= size || y < 0 || y >= size) {
        return 0.0; // TODO
    }
    return height_input_buffer.heightmap[y * size + x];
}

void main() {
    uvec3 gid = gl_GlobalInvocationID;
    // int width = int(gl_WorkGroupSize.x * gl_NumWorkGroups.x);
    // int height = int(gl_WorkGroupSize.y * gl_NumWorkGroups.y);
    int size = 3;
    int width = size;
    int height = width;

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
    float velocity = velocity_buffer.velocity[y * width + x];

    // Update logic here
    // ...

    // For example, update height based on some factor of neighboring heights and current velocity
    velocity_buffer.velocity[y * size + x] += hCurrent + 0.1 * (hLeft + hRight + hUp + hDown - 4.0 * hCurrent);

    // Update height
    height_output_buffer.heightmap[y * width + x] += velocity_buffer.velocity[y * size + x] * 0.01;

    // Similarly, you can update the velocity_data_buffer as per your simulation needs
}
