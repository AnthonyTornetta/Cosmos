package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;

public class LightModel extends CubeModel
{
	@Override
	public float u(BlockFace side)
	{
		return 0;
	}

	@Override
	public float v(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS;
	}
}
