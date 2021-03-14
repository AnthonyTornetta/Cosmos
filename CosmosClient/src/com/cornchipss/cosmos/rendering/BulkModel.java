package com.cornchipss.cosmos.rendering;

import java.util.HashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;

import com.cornchipss.cosmos.blocks.BlockFace;
import com.cornchipss.cosmos.lights.LightMap;
import com.cornchipss.cosmos.material.Material;
import com.cornchipss.cosmos.material.Materials;
import com.cornchipss.cosmos.models.AnimatedCubeModel;
import com.cornchipss.cosmos.models.CubeModel;
import com.cornchipss.cosmos.models.IHasModel;

public class BulkModel
{
	private IHasModel[][][] cubes;
	
	public void setModels(IHasModel[][][] blocks)
	{
		this.cubes = blocks;
	}	
	
	private static class MaterialMeshGenerator
	{
		List<Integer> indicies = new LinkedList<>();
		List<Float> verticies = new LinkedList<>();
		List<Float> uvs = new LinkedList<>();
		List<Float> lights = new LinkedList<>();
		
		/**
		 * Only used for an animated material
		 * TODO: put this class in the Material and make it modifyable by the material class so I don't have to hardcode this stuff here
		 */
		List<Float> animationInfo; // only initialized if it is an animated material
		
		int maxIndex = 0;
	}
	
	private List<MaterialMesh> meshes;
	
	private Map<Material, MaterialMeshGenerator> indevMeshes;
	
	public BulkModel(IHasModel[][][] models)
	{
		cubes = models;
		
		meshes = new LinkedList<>();
		
		indevMeshes = new HashMap<>();
	}
	
	boolean within(int x, int y, int z)
	{
		return z >= 0 && z < cubes.length
				&& y >= 0 && y < cubes[z].length
				&& x >= 0 && x < cubes[z][y].length;
	}
	
	private void doStuff(LightMap lightMap, 
			int x, int y, int z, 
			int dx, int dy, int dz, 
			int offX, int offY, int offZ, 
			BlockFace face, MaterialMeshGenerator matMesh)
	{
		CubeModel model = cubes[z][y][x].model();
		boolean animated = model instanceof AnimatedCubeModel;
		
		for(float f : model.verticies(face, x, y, z))
			matMesh.verticies.add(f);
		
		matMesh.maxIndex = indiciesAndUvs(face, model, matMesh);
		
		if(animated)
		{
			AnimatedCubeModel modelAnimated = (AnimatedCubeModel)model;
			for(int i = 0; i < 4; i++)
			{
				matMesh.animationInfo.add((float)modelAnimated.maxAnimationStage(face));
				matMesh.animationInfo.add(modelAnimated.animationDelay(face) * 1000);
			}
		}
		
		lighting(offX, offY, offZ, x + dx, y + dy, z + dz, lightMap, matMesh);
	}
	
	private void computeEverything(BulkModel left, BulkModel right, BulkModel top, 
			BulkModel bottom, BulkModel front, BulkModel back,
			int offX, int offY, int offZ, LightMap lightMap)
	{
		for(int z = 0; z < length(); z++)
		{
			for(int y = 0; y < height(); y++)
			{
				for(int x = 0; x < width(); x++)
				{
					if(cubes[z][y][x] != null)
					{
						boolean withinB;
						
						Material mat = cubes[z][y][x].model().material();
						
						boolean animated = Materials.ANIMATED_DEFAULT_MATERIAL.equals(mat);
						
						if(!indevMeshes.containsKey(mat))
						{
							MaterialMeshGenerator gen = new MaterialMeshGenerator();
							indevMeshes.put(mat, gen);
							if(animated)
							{
								gen.animationInfo = new LinkedList<>();
							}
						}
						
						MaterialMeshGenerator matMesh = indevMeshes.get(mat);
						
						if((!(withinB = within(x, y + 1, z)) &&
							(top == null || top.cubes[z][0][x] == null)) 
								|| withinB && cubes[z][y + 1][x] == null)
						{
							doStuff(lightMap, x, y, z, 0, 1, 0, offX, offY, offZ, BlockFace.TOP, matMesh);
						}
						if((!(withinB = within(x, y - 1, z)) &&
								(bottom == null || bottom.cubes[z][bottom.height() - 1][x] == null)) 
									|| withinB && cubes[z][y - 1][x] == null)
						{
							doStuff(lightMap, x, y, z, 0, -1, 0, offX, offY, offZ, BlockFace.BOTTOM, matMesh);
						}
						
						if((!(withinB = within(x, y, z + 1)) &&
								(front == null || front.cubes[0][y][x] == null)) 
									|| withinB && cubes[z + 1][y][x] == null)
						{
							doStuff(lightMap, x, y, z, 0, 0, 1, offX, offY, offZ, BlockFace.FRONT, matMesh);
						}
						if((!(withinB = within(x, y, z - 1)) &&
								(back == null || back.cubes[back.length() - 1][y][x] == null)) 
									|| withinB && cubes[z - 1][y][x] == null)
						{
							doStuff(lightMap, x, y, z, 0, 0, -1, offX, offY, offZ, BlockFace.BACK, matMesh);
						}
						

						if((!(withinB = within(x + 1, y, z)) &&
								(right == null || right.cubes[z][y][0] == null)) 
									|| withinB && cubes[z][y][x + 1] == null)
						{
							doStuff(lightMap, x, y, z, 1, 0, 0, offX, offY, offZ, BlockFace.RIGHT, matMesh);
						}
						if((!(withinB = within(x - 1, y, z)) &&
								(left == null || left.cubes[z][y][left.width() - 1] == null)) 
									|| withinB && cubes[z][y][x - 1] == null)
						{
							doStuff(lightMap, x, y, z, -1, 0, 0, offX, offY, offZ, BlockFace.LEFT, matMesh);
						}
					}
				}
			}
		}
	}
	
	private void lighting(int offX, int offY, int offZ, int x, int y, int z, LightMap lightMap, MaterialMeshGenerator matMesh)
	{
		float col = 0;
		if(lightMap.within(offX + x, offY + y, offZ + z))
			col = lightMap.at(x, y, z, offX, offY, offZ);
		
		matMesh.lights.add(col);
		matMesh.lights.add(col);
		matMesh.lights.add(col);
		
		matMesh.lights.add(col);
		matMesh.lights.add(col);
		matMesh.lights.add(col);
		
		matMesh.lights.add(col);
		matMesh.lights.add(col);
		matMesh.lights.add(col);
		
		matMesh.lights.add(col);
		matMesh.lights.add(col);
		matMesh.lights.add(col);
	}
	
	private int indiciesAndUvs(BlockFace side, CubeModel model, MaterialMeshGenerator matMesh)
	{
		int[] indiciesArr = model.indicies(side);
		int max = -1;
		
		for(int index : indiciesArr)
		{
			matMesh.indicies.add(index + matMesh.maxIndex);
			if(max < index)
				max = index;
		}
		  
		float u = model.u(side);
		float v = model.v(side);
		
		float uEnd = u + CubeModel.TEXTURE_DIMENSIONS;
		float vEnd = v + CubeModel.TEXTURE_DIMENSIONS;
		
		matMesh.uvs.add(uEnd);
		matMesh.uvs.add(vEnd);
		
		matMesh.uvs.add(uEnd);
		matMesh.uvs.add(v);
		
		matMesh.uvs.add(u);
		matMesh.uvs.add(v);

		matMesh.uvs.add(u);
		matMesh.uvs.add(vEnd);
		
		return matMesh.maxIndex + max + 1;
	}
	
	/**
	 * algorithm kinda
	 */
	public void render(BulkModel left, BulkModel right, BulkModel top, 
			BulkModel bottom, BulkModel front, BulkModel back,
			int offX, int offY, int offZ, LightMap lightMap)
	{
		indevMeshes.clear();
		meshes.clear();
		
		computeEverything(left, right, top, bottom, front, back, offX, offY, offZ, lightMap);
		
		for(Material m : indevMeshes.keySet())
		{
			MaterialMeshGenerator matMesh = indevMeshes.get(m);
			
			int i = 0;
			int[] indiciesArr = new int[matMesh.indicies.size()];
			for(int index : matMesh.indicies)
				indiciesArr[i++] = index;
			
			i = 0;
			float[] verticiesArr = new float[matMesh.verticies.size()];
			
	//		float dz = cubes.length / 2.0f;
	//		float dy = cubes[(int)dz].length / 2.0f;
	//		float dx = cubes[(int)dz][(int)dy].length / 2.0f;
			
			// verticies must be in the order of x,y,z
			for(float vertex : matMesh.verticies)
				verticiesArr[i++] = vertex;// - (i % 3 == 0 ? dx : (i % 3 == 1 ? dy : dz)); // centers everything around the center of the bulk model's 0,0
			
			i = 0;
			float[] uvsArr = new float[matMesh.uvs.size()];
			for(float uv : matMesh.uvs)
				uvsArr[i++] = uv;
			
			i = 0;
			float[] lightsArr = new float[matMesh.lights.size()];
			for(float l : matMesh.lights)
				lightsArr[i++] = l;
			
			boolean isAnimated = matMesh.animationInfo != null;
			
			Mesh mesh = Mesh.createMesh(verticiesArr, indiciesArr, uvsArr, lightsArr, !isAnimated);
			
			if(isAnimated)
			{
				float[] animationInfoArr = new float[matMesh.animationInfo.size()];
				i = 0;
				for(float f : matMesh.animationInfo)
					animationInfoArr[i++] = f;
				
				mesh.storeData(4, 2, animationInfoArr);
				
				mesh.unbind();
			}
			
			meshes.add(new MaterialMesh(m, mesh));
		}
	}
	
	public List<MaterialMesh> materialMeshes()
	{
		return meshes;
	}
	
	public int width()
	{
		return cubes[0][0].length;
	}
	
	public int height()
	{
		return cubes[0].length;
	}
	
	public int length()
	{
		return cubes.length;
	}
}
