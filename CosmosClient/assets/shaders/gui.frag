#version 330 core

in vec2 frag_uv;

uniform sampler2D sampler;

out vec4 FragColor;

void main()
{
	vec4 textColor = texture(sampler, vec2(frag_uv.x, frag_uv.y));
	
	if(textColor[3] < 0.1)
		discard;
	
	FragColor = textColor;
}