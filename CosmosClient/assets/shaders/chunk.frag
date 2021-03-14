#version 330 core

in vec3 frag_pos;
in vec3 frag_color;
in vec2 frag_uv;
in vec3 frag_light;

uniform sampler2D sampler;

out vec4 FragColor;

uniform float u_ambientLight;

uniform float u_time;

void main()
{
	vec4 textColor = 
		vec4(max(frag_light.x, u_ambientLight), max(frag_light.y, u_ambientLight), max(frag_light.z, u_ambientLight), 1)
		* texture(sampler, vec2(frag_uv.x, frag_uv.y));
	
	FragColor = textColor;
}