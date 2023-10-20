#version 450

layout(location = 0) in vec2 vertex_position;
layout(location = 0) out vec4 frag_color;

uniform sampler2D input_texture;

void main() {
    gl_Position = vec4(vertex_position, 0.0, 1.0);
    vec4 color = texture(input_texture, vertex_position * 0.5 + 0.5);
    frag_color = color;
}
