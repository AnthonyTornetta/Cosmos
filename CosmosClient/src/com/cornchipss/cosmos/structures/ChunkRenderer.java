package com.cornchipss.cosmos.structures;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.lights.LightMap;
import com.cornchipss.cosmos.models.IHasModel;
import com.cornchipss.cosmos.rendering.BulkModel;
import com.cornchipss.cosmos.world.Chunk;

public class ChunkRenderer implements IChunkHandler
{
	private BulkModel bm;
	
	private boolean rendered = false;
	
	private Chunk c;
	private LightMap map;
	private ChunkRenderer[][][] renderers;
	
	public ChunkRenderer(Chunk c, LightMap map, ChunkRenderer[][][] renderers)
	{
		bm = new BulkModel(new IHasModel[Chunk.LENGTH][Chunk.HEIGHT][Chunk.WIDTH]);
		
		this.c = c;
		this.map = map;
	}
	
	@Override
	public void onBlockUpdate(Structure s, Chunk c, int x, int y, int z, Block oldBlock, Block newBlock)
	{
		
	}
	
	private BulkModel left()
	{
		if(c.index().x() == 0)
			return null;
		else
			return renderers[c.index().z()][c.index().y()][c.index().x() - 1].bulkModel();
	}
	
	private BulkModel right()
	{
		if(c.index().x() == renderers[c.index().z()][c.index().y()].length - 1)
			return null;
		else
			return renderers[c.index().z()][c.index().y()][c.index().x() + 1].bulkModel();
	}
	
	private BulkModel bottom()
	{
		if(c.index().y() == 0)
			return null;
		else
			return renderers[c.index().z()][c.index().y() - 1][c.index().x()].bulkModel();
	}
	
	private BulkModel top()
	{
		if(c.index().y() == renderers[c.index().z()].length - 1)
			return null;
		else
			return renderers[c.index().z()][c.index().y() + 1][c.index().x()].bulkModel();
	}
	
	private BulkModel back()
	{
		if(c.index().z() == 0)
			return null;
		else
			return renderers[c.index().z() - 1][c.index().y()][c.index().x()].bulkModel();
	}
	
	private BulkModel front()
	{
		if(c.index().z() == renderers[c.index().z()].length - 1)
			return null;
		else
			return renderers[c.index().z() + 1][c.index().y()][c.index().x()].bulkModel();
	}
	
	public void render()
	{
		rendered = true;
		
		bm.render(
				left(), right(), top(), 
				bottom(), front(), back(), 
				c.offset().x(), c.offset().y(), c.offset().z(), map);
		
//		model.render(
//				leftNeighbor() != null ? leftNeighbor().model : null, 
//				rightNeighbor() != null ? rightNeighbor().model : null, 
//				topNeighbor() != null ? topNeighbor().model : null, 
//				bottomNeighbor() != null ? bottomNeighbor().model : null, 
//				frontNeighbor() != null ? frontNeighbor().model : null, 
//				backNeighbor() != null ? backNeighbor().model : null,
//						offset().x(), offset().y(), offset().z(), structure().lightMap());
	}
	
	public BulkModel bulkModel()
	{
		return bm;
	}
}
