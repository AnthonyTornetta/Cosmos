package com.cornchipss.cosmos.world;

import java.util.List;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.blocks.ClientBlock;
import com.cornchipss.cosmos.blocks.LitBlock;
import com.cornchipss.cosmos.rendering.BulkModel;
import com.cornchipss.cosmos.rendering.MaterialMesh;
import com.cornchipss.cosmos.structures.ClientStructure;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.Utils;

public class ClientChunk extends Chunk
{
	private BulkModel model;
	
	private boolean rendered;
	
	public ClientChunk(int offX, int offY, int offZ, Structure s)
	{
		super(offX, offY, offZ, s);
		
		model = new BulkModel(blocks());
	}
	
	@Override
	protected ClientBlock[][][] createBlocksArray(int w, int h, int l)
	{
		return new ClientBlock[l][h][w];
	}
	
	/**
	 * <p>Converts the blocks into a drawable mesh accessible through {@link Chunk#mesh()}</p>
	 * <p>Once this method is called, all changes to this chunk's blocks will call this method.</p>
	 */
	public void render()
	{
		rendered = true;
		
		model.render(
				leftNeighbor() != null ? leftNeighbor().model : null, 
				rightNeighbor() != null ? rightNeighbor().model : null, 
				topNeighbor() != null ? topNeighbor().model : null, 
				bottomNeighbor() != null ? bottomNeighbor().model : null, 
				frontNeighbor() != null ? frontNeighbor().model : null, 
				backNeighbor() != null ? backNeighbor().model : null,
						offset().x(), offset().y(), offset().z(), structure().lightMap());
	}
	
	/**
	 * Gets the block relative to this chunk's position
	 * @param x The X coordinate relative to the chunk's position
	 * @param y The Y coordinate relative to the chunk's position
	 * @param z The Z coordinate relative to the chunk's position
	 * @return The block at this position - null if there is no block
	 */
	@Override
	public ClientBlock block(int x, int y, int z)
	{
		return (ClientBlock)super.block(x, y, z);
	}
	
	/**
	 * <p>Sets the block at the given coordinates relative to this chunk.</p>
	 * <p>If {@link Chunk#render()} has been called previously, this will also call it.</p>
	 * @param x The X coordinate relative to this chunk
	 * @param y The Y coordinate relative to this chunk
	 * @param z The Z coordinate relative to this chunk
	 * @param block The block to set it to
	 * @param shouldRender If true, the chunk will update the lightmap + re-render the mesh. If the chunk has not been rendered yet - this will be false no matter what is passed
	 */
	@Override
	public void block(int x, int y, int z, Block block, boolean shouldRender)
	{	
		if(!within(x, y, z))
			throw new IllegalArgumentException("Bad x,y,z: " + x + ", " + y + ", " + z);
		
		if(block != null && !(block instanceof ClientBlock))
			throw new IllegalArgumentException("Block must be instanceof ClientBlock!");
		
		shouldRender = rendered && shouldRender;
		
		if(!Utils.equals(blocks()[z][y][x], block))
		{
			blocks()[z][y][x] = (ClientBlock)block;
			
			if(block != null)
				structure().lightMap().addBlocking(x + offset().x(), y + offset().y(), z + offset().z());
			else
				structure().lightMap().removeBlocking(x + offset().x(), y + offset().y(), z + offset().z());
			
			if(block instanceof LitBlock)
			{
				// remove it if there is already one
				structure().lightMap().removeLightSource(x + offset().x(), y + offset().y(), z + offset().z());
				
				structure().lightMap().lightSource(x + offset().x(), y + offset().y(), z + offset().z(), 
							((LitBlock) block).lightSource());
				
				if(shouldRender)
				{
					structure().calculateLights(true);
				}
			}
			else if(structure().lightMap().hasLightSource(x + offset().x(), y + offset().y(), z + offset().z()))
			{
				structure().lightMap().removeLightSource(x + offset().x(), y + offset().y(), z + offset().z());
				
				if(shouldRender)
					structure().calculateLights(true);
			}
			else if(shouldRender)
			{
				structure().calculateLights(true);
			}
			
			if(shouldRender) // only if the chunk has been rendered at least 1 time before
				render(); // update the chunk's model for the new block
			
			// Make sure if this block is neighboring another chunk, that chunk updates aswell
			if(x == 0 && leftNeighbor() != null && shouldRender)
				leftNeighbor().render();
			if(x + 1 == Chunk.WIDTH && rightNeighbor() != null && shouldRender)
				rightNeighbor().render();
			
			if(y == 0 && bottomNeighbor() != null && shouldRender)
				bottomNeighbor().render();
			if(y + 1 == Chunk.HEIGHT && topNeighbor() != null && shouldRender)
				topNeighbor().render();
			
			if(z == 0 && backNeighbor() != null && shouldRender)
				backNeighbor().render();
			if(z + 1 == Chunk.LENGTH && frontNeighbor() != null && shouldRender)
				frontNeighbor().render();
		}
	}
	
	public BulkModel model()
	{
		return model;
	}
	
	/**
	 * The mesh of all the blocks - null if {@link Chunk#render()} has not been called.
	 * @return The mesh of all the blocks - null if {@link Chunk#render()} has not been called.
	 */
	public List<MaterialMesh> meshes()
	{
		return model.materialMeshes();
	}
	
	@Override
	public ClientStructure structure()
	{
		return (ClientStructure)super.structure();
	}
	
	@Override
	public ClientChunk leftNeighbor()
	{
		return (ClientChunk)super.leftNeighbor();
	}
	@Override
	public ClientChunk rightNeighbor()
	{
		return (ClientChunk)super.rightNeighbor();
	}
	@Override
	public ClientChunk topNeighbor()
	{
		return (ClientChunk)super.topNeighbor();
	}
	@Override
	public ClientChunk bottomNeighbor()
	{
		return (ClientChunk)super.bottomNeighbor();
	}
	@Override
	public ClientChunk frontNeighbor()
	{
		return (ClientChunk)super.frontNeighbor();
	}
	@Override
	public ClientChunk backNeighbor()
	{
		return (ClientChunk)super.backNeighbor();
	}
	
	
	@Override
	protected ClientBlock[][][] blocks()
	{
		return (ClientBlock[][][])super.blocks();
	}
}
