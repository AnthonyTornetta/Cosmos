#version 330 core

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inColor;
layout (location = 2) in vec2 inUv;
layout (location = 3) in vec3 inLight;

uniform mat4 u_camera;
uniform mat4 u_proj;

uniform mat4 u_transform;

out vec3 frag_pos;
out vec3 frag_color;
out vec2 frag_uv;
out vec3 frag_light;

void main()
{
	frag_pos = inPos;
	frag_color = inColor;
	frag_uv = inUv;
	frag_light = inLight;
	
	gl_Position = u_proj * u_camera * u_transform * vec4(inPos, 1.0);
}