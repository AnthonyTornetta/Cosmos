package com.cornchipss.cosmos.blocks;

import com.cornchipss.cosmos.blocks.data.BlockData;
import com.cornchipss.cosmos.structures.Structure;

/**
 * <p>A block in the world</p>
 * <p>Only one instance of each block should ever be present</p>
 * <p>Each block of the same type in the world points to that instance</p>
 * <p>Use {@link IHasData} to differentiate between different blocks</p>
 */
public class Block
{
	private short id;
	
	private String name;
	
	/**
	 * <p>A block in the world</p>
	 * <p>Only one instance of each block should ever be present</p>
	 * <p>Each block of the same type in the world points to that instance</p>
	 * <p>Use {@link BlockData} to differentiate between different blocks</p>
	 * 
	 * @param m The model the block has
	 * @param name The name used to refer to the block in the registry
	 */
	public Block(String name)
	{
		this.name = name;
		id = -1;
	}
	
	public short numericId()
	{
		if(id == -1)
			throw new IllegalStateException("Id of a block was asked for before the block was initialized");
		
		return id;
	}
	
	public void blockId(short s)
	{
		if(id != -1)
			throw new IllegalStateException("Id of a block cannot be set more than once!!!");
		
		id = s;
	}
	
	/**
	 * Determines whether this block can be added to a given structure
	 * @param s The structure to check
	 * @return True if it can be added, false if not
	 */
	public boolean canAddTo(Structure s)
	{
		return true;
	}
	
	@Override
	public boolean equals(Object o)
	{
		if(o instanceof Block)
		{
			return ((Block)o).numericId() == numericId();
		}
		
		return false;
	}
	
	@Override
	public int hashCode()
	{
		return numericId();
	}

	public String name()
	{
		return name;
	}
}
