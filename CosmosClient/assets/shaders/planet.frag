#version 400 core

uniform float u_time;
uniform sampler2D sampler;

in vec3 color;
in vec2 uv;

out vec4 out_color;

void main()
{
	vec4 textColor = texture(sampler, uv);
	
	if(textColor.a < 0.1)
		discard;
	
	out_color = textColor * vec4(color, 1.0);
}
