package com.cornchipss.cosmos.blocks;

import com.cornchipss.cosmos.blocks.data.BlockData;
import com.cornchipss.cosmos.structures.Ship;
import com.cornchipss.cosmos.structures.Structure;

/**
 * A block that is unique to ships (see {@link Ship})
 */
public class ShipBlock extends Block
{
	/**
	 * <p>A block in the world that can only be placed on a {@link Ship}</p>
	 * <p>Only one instance of each block should ever be present</p>
	 * <p>Each block of the same type in the world points to that instance</p>
	 * <p>Use {@link BlockData} to differentiate between different blocks</p>
	 * 
	 * @param m The model the block has
	 * @param name The name used to refer to the block in the registry
	 */
	public ShipBlock(String name)
	{
		super(name);
	}
	
	@Override
	public boolean canAddTo(Structure s)
	{
		return s instanceof Ship;
	}
}
