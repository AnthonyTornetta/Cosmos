package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;

public abstract class AnimatedTextureModel extends CubeModel
{
	public abstract int maxStage();
	
	public abstract float u(BlockFace s, int stage);
	public abstract float v(BlockFace s, int stage);
	
	@Override
	public float u(BlockFace side)
	{
		return u(side, 0);
	}

	@Override
	public float v(BlockFace side)
	{
		return v(side, 0);
	}
}
