package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;
import com.cornchipss.cosmos.material.Material;
import com.cornchipss.cosmos.material.Materials;

public class ShipCoreModel extends AnimatedCubeModel
{
	@Override
	public float u(BlockFace side)
	{
		return 0;
	}
	
	@Override
	public float v(BlockFace side)
	{
		return 0;
	}
	
	@Override
	public int maxAnimationStage(BlockFace side)
	{
		return 14;
	}

	@Override
	public float animationDelay(BlockFace side)
	{
		return 1/10.0f;
	}
	
	@Override
	public Material material()
	{
		return Materials.ANIMATED_DEFAULT_MATERIAL;
	}
}
