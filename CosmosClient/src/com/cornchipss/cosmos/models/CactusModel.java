package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;

public class CactusModel extends CubeModel
{
	@Override
	public float u(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS * 10;
	}

	@Override
	public float v(BlockFace side)
	{
		return 0;
	}

}
