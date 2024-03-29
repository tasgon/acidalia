#version 450

layout(location=0) in vec2 v_TexCoords;
layout(location=0) out vec4 f_Color;

layout(set = 0, binding = 0) uniform texture2D t_Diffuse;
layout(set = 0, binding = 1) uniform sampler s_Diffuse;

void main() {
    f_Color = texture(sampler2D(t_Diffuse, s_Diffuse), v_TexCoords);
}