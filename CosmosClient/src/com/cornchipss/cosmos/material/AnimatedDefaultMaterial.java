package com.cornchipss.cosmos.material;

import org.joml.Matrix4fc;
import org.lwjgl.glfw.GLFW;

public class AnimatedDefaultMaterial extends Material
{
	private int projLoc, camLoc, transLoc, stateLoc, ambientLoc;
	
	public AnimatedDefaultMaterial()
	{
		super("assets/shaders/chunk-animated", "assets/images/atlas/main-animated");
	}

	@Override
	public void initUniforms(Matrix4fc projectionMatrix, Matrix4fc camera, Matrix4fc transform, boolean inGUI)
	{
		shader().setUniformMatrix(projLoc, projectionMatrix);
		shader().setUniformMatrix(camLoc, camera);
		shader().setUniformMatrix(transLoc, transform);
		shader().setUniformF(ambientLoc, inGUI ? 1 : 0.2f);
		shader().setUniformI(stateLoc, (int)(GLFW.glfwGetTime() * 1000));
	}

	@Override
	protected void initShader()
	{
		projLoc = shader().uniformLocation("u_proj");
		camLoc = shader().uniformLocation("u_camera");
		transLoc = shader().uniformLocation("u_transform");
		stateLoc = shader().uniformLocation("u_animation_state");
		ambientLoc = shader().uniformLocation("u_ambientLight");
	}
}
