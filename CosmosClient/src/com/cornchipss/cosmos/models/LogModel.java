package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;

public class LogModel extends CubeModel
{
	@Override
	public float u(BlockFace side)
	{
		switch(side)
		{
		case TOP:
			return 1 * CubeModel.TEXTURE_DIMENSIONS;
		case BOTTOM:
			return 1 * CubeModel.TEXTURE_DIMENSIONS;
		default:
			return 2 * CubeModel.TEXTURE_DIMENSIONS;
		}
	}

	@Override
	public float v(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS;
	}
}
