package com.cornchipss.cosmos.blocks;

import java.util.List;

/**
 * The instance of every block in the base game
 */
public class Blocks
{
	private static List<Block> allBlocks;
	
	/**
	 * Adds all the blocks to a list
	 */
	public static void addBlock(Block b)
	{
		allBlocks.add(b);
	}
	
	public static Block fromNumericId(short id)
	{
		if(id == 0)
			return null;
		return allBlocks.get(id - 1);
	}
	
	/**
	 * Returns a list of all the blocks in the game - this cannot be modified.
	 * @return a list of all the blocks in the game - this cannot be modified.
	 */
	public static List<Block> all()
	{
		return allBlocks;
	}
}
