package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;

public class LeafModel extends CubeModel
{
	@Override
	public float u(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS * 3;
	}

	@Override
	public float v(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS;
	}
}
