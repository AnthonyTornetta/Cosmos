package com.cornchipss.cosmos.material;

import org.joml.Matrix4fc;

import com.cornchipss.cosmos.rendering.Texture;
import com.cornchipss.cosmos.shaders.Shader;

public abstract class Material
{
	private Shader shader;
	private String textureLoc;
	
	private Texture texture;
	
	public Material(String shaderLoc, String textureLoc)
	{
		this(new Shader(shaderLoc), textureLoc);
	}
	
	public Material(Shader s, String t)
	{
		shader = s;
		textureLoc = t;
	}
	
	public Material(Shader s, Texture texture)
	{
		shader = s;
		this.texture = texture;
	}
	
	public abstract void initUniforms(
			Matrix4fc projectionMatrix, Matrix4fc camera, 
			Matrix4fc transform, boolean inGUI);

	/**
	 * Used to get the uniform locations
	 */
	protected abstract void initShader();
	
	public Shader shader()
	{
		return shader;
	}
	
	public Texture texture()
	{
		return texture;
	}
	
	public void useShader()
	{
		shader.use();
	}
	
	public void stopShader()
	{
		shader.stop();
	}
	
	public void bindTexture()
	{
		texture.bind();
	}
	
	public void use()
	{
		shader.use();
		texture.bind();
	}
	
	public void stop()
	{
		Texture.unbind();
		shader.stop();
	}

	public void init()
	{
		shader.init();
		initShader();
		
		if(texture == null)
			texture = Texture.loadTexture(textureLoc);
	}
}
