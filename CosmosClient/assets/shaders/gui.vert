#version 330 core

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec3 inColor;
layout (location = 2) in vec2 inUv;

uniform mat4 u_transform;
uniform mat4 u_projection;

out vec2 frag_uv;

void main()
{
	frag_uv = inUv;
	
	gl_Position = u_projection * u_transform * vec4(inPos, 1.0);
}