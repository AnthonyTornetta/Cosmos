package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;

public class ShipHullModel extends CubeModel
{
	@Override
	public float u(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS * 4;
	}

	@Override
	public float v(BlockFace side)
	{
		return CubeModel.TEXTURE_DIMENSIONS;
	}
}
