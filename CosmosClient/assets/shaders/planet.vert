#version 400 core

uniform float u_time;

uniform mat4 projection;
uniform mat4 view;

uniform mat4 u_transformation_matrix;

// For animated textures (todo for later)
uniform float u_max_uv;
uniform float u_time_between_uv;
uniform float u_uv_size;
uniform float u_uv_width;

layout (location = 0) in vec3 in_position;
layout (location = 1) in vec3 in_color;
layout (location = 2) in vec2 in_uv;
layout (location = 3) in vec3 in_translation;

out vec3 color;
out vec2 uv;

void main()
{
	//float icr = (in_uv.x - u_max_uv) / u_uv_size;

	//float deltaUv = icr * ((u_time / (u_time_between_uv * icr)) % icr);

	//uv = vec2((in_uv.x + deltaUv) % u_uv_width, in_uv.y + int((in_uv.x + deltaUv) / u_uv_width));

	uv = in_uv;

	vec4 relativeToCamera = view * u_transformation_matrix * vec4(in_translation + in_position, 1.0);

	float dist = -relativeToCamera.z;
	float col = max(1 - dist / 1000.0f, 0);
	//col = 0;
	color = in_color * vec3(col, col, col);

	gl_Position = projection * relativeToCamera;
}
