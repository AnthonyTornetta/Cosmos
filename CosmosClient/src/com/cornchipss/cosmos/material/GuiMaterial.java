package com.cornchipss.cosmos.material;

import org.joml.Matrix4fc;

public class GuiMaterial extends Material
{
	private int guiProjLoc, guiTransLoc;
	
	public GuiMaterial()
	{
		super("assets/shaders/gui", "assets/images/atlas/gui");
	}

	@Override
	public void initUniforms(Matrix4fc projectionMatrix, Matrix4fc cam, Matrix4fc transform, boolean isGUI)
	{
		shader().setUniformMatrix(guiTransLoc, transform);
		shader().setUniformMatrix(guiProjLoc, projectionMatrix);
	}

	@Override
	protected void initShader()
	{
		guiTransLoc = shader().uniformLocation("u_transform");
		guiProjLoc = shader().uniformLocation("u_projection");
	}
}