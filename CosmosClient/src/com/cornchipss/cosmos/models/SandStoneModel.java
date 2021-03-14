package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;

public class SandStoneModel extends CubeModel
{
	@Override
	public float u(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS * 8;
	}

	@Override
	public float v(BlockFace side)
	{
		return 0;
	}

}
