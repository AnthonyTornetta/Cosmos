#version 400 core

uniform mat4 projection;
uniform mat4 view;

uniform mat4 u_transformation_matrix;

layout (location = 0) in vec3 in_position;
layout (location = 1) in vec3 in_color;

out vec3 color;

void main()
{
	gl_Position = projection * view * u_transformation_matrix * vec4(in_position, 1.0);

	color = in_color;
}
