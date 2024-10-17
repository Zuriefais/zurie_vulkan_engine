#version 450

layout(location = 0) out vec4 f_color;
layout(location = 0) in vec4 frag_color;

void main() {
    f_color = frag_color;
}
