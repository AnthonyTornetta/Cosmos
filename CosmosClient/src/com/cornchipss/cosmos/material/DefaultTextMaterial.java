package com.cornchipss.cosmos.material;

import org.joml.Matrix4fc;

import com.cornchipss.cosmos.rendering.Texture;
import com.cornchipss.cosmos.shaders.Shader;

// TODO: fix how this works
public class DefaultTextMaterial extends Material
{
	public DefaultTextMaterial(Shader s, Texture t)
	{
		super(s, t);
	}

	@Override
	public void initUniforms(Matrix4fc projectionMatrix, Matrix4fc matrix4fc, Matrix4fc transform, boolean inGUI)
	{
		// not needed.
	}

	@Override
	protected void initShader()
	{
		// not needed.
	}
}
