package com.cornchipss.cosmos.rendering;

import com.cornchipss.cosmos.material.Material;

/**
 * It's a mesh and a material.
 */
public class MaterialMesh
{
	private Material mat;
	private Mesh mesh;
	
	public MaterialMesh(Material mat, Mesh mesh)
	{
		this.mat = mat;
		this.mesh = mesh;
	}
	
	public Material material() { return mat; }
	public Mesh mesh() { return mesh; }
}
