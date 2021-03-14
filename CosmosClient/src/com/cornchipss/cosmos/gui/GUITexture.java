package com.cornchipss.cosmos.gui;

import org.joml.Vector3fc;

import com.cornchipss.cosmos.rendering.Mesh;

public class GUITexture extends GUIElement
{
	private static final float UV_WIDTH = 0.5f;
	private static final float UV_HEIGHT = 0.5f;
	
	public static final int[] indices = new int[]
			{
				0, 1, 3,
				1, 2, 3
			};
	
	public static float[] makeVerts(float w, float h)
	{
		return new float[]
			{
				 w,  h, 0,  // top right
				 w,  0, 0,  // bottom right
			     0,  0, 0,  // bottom left
			     0,  h, 0   // top left 
			};
	}
	
	public static float[] makeUVs(float u, float v)
	{
		return new float[]
			{
				u + UV_WIDTH, v,
				u + UV_WIDTH, v + UV_HEIGHT,
				u, v + UV_HEIGHT,
				u, v
			};
	}
	
	private Mesh guiMesh;

	public GUITexture(Vector3fc position, float w, float h, float u, float v)
	{
		super(position);
		guiMesh = Mesh.createMesh(makeVerts(w, h), indices, makeUVs(u, v));
	}
	
	@Override
	public Mesh guiMesh()
	{
		return guiMesh;
	}
}
