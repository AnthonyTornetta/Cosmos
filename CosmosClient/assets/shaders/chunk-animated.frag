#version 330 core

in vec3 frag_pos;
in vec3 frag_color;
in vec2 frag_uv;
in vec3 frag_light;
in vec2 frag_animation;

uniform int u_animation_state; // actually an int

uniform sampler2D sampler;

out vec4 FragColor;

uniform float u_ambientLight;

void main()
{
	int numFrames = int(frag_animation.x);
	int delay = int(frag_animation.y);
	float textureWidth = 16.0f; // TODO: pass this in via uniform

	vec4 textColor = 
		vec4(max(frag_light.x, u_ambientLight), max(frag_light.y, u_ambientLight), max(frag_light.z, u_ambientLight), 1)
		* texture(sampler, vec2(frag_uv.x + ((u_animation_state / delay % numFrames) / textureWidth), frag_uv.y));
	
	FragColor = textColor;
}