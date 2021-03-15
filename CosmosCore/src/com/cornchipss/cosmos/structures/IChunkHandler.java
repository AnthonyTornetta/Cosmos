package com.cornchipss.cosmos.structures;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.world.Chunk;

public interface IChunkHandler
{
	public void onBlockUpdate(Structure s, Chunk c, int x, int y, int z, Block oldBlock, Block newBlock);
}
