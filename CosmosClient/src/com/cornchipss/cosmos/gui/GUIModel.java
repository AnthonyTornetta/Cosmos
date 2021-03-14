package com.cornchipss.cosmos.gui;

import org.joml.Matrix4f;
import org.joml.Vector3fc;

import com.cornchipss.cosmos.blocks.BlockFace;
import com.cornchipss.cosmos.material.Material;
import com.cornchipss.cosmos.models.CubeModel;
import com.cornchipss.cosmos.rendering.Mesh;
import com.cornchipss.cosmos.utils.Maths;

public class GUIModel extends GUIElement
{
	private Mesh mesh;
	private Material mat;
	
	public GUIModel(Vector3fc position, float scale, CubeModel model)
	{
		this(Maths.createTransformationMatrix(position, 0, 0, 0, scale), model);
	}
	
	public GUIModel(Matrix4f transform, CubeModel m)
	{
		this(transform, m.createMesh(0, 0, -1, 1, BlockFace.FRONT), m.material());
	}
	
	public GUIModel(Vector3fc position, float scale, Mesh m, Material mat)
	{
		this(Maths.createTransformationMatrix(position, 0, 0, 0, scale), m, mat);
	}
	
	public GUIModel(Matrix4f transform, Mesh m, Material mat)
	{
		super(transform);
		this.mesh = m;
		this.mat = mat;
	}
	
	@Override
	public Mesh guiMesh()
	{
		return mesh;
	}
	
	@Override
	public Material material()
	{
		return mat;
	}
}
