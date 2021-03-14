package com.cornchipss.cosmos.material;

import org.joml.Matrix4fc;
import org.lwjgl.glfw.GLFW;

public class DefaultMaterial extends Material
{
	private int projLoc, camLoc, transLoc, ambientLoc, timeLoc;
	
	public DefaultMaterial()
	{
		super("assets/shaders/chunk", "assets/images/atlas/main");
	}

	@Override
	public void initUniforms(Matrix4fc projectionMatrix, Matrix4fc camera, Matrix4fc transform, boolean inGUI)
	{
		shader().setUniformMatrix(projLoc, projectionMatrix);
		shader().setUniformMatrix(camLoc, camera);
		shader().setUniformMatrix(transLoc, transform);
		shader().setUniformF(ambientLoc, inGUI ? 1 : 0.2f);
		shader().setUniformF(timeLoc, (float)GLFW.glfwGetTime());
	}

	@Override
	protected void initShader()
	{
		projLoc = shader().uniformLocation("u_proj");
		camLoc = shader().uniformLocation("u_camera");
		transLoc = shader().uniformLocation("u_transform");
		ambientLoc = shader().uniformLocation("u_ambientLight");
		timeLoc = shader().uniformLocation("u_time");
	}
}
