package com.cornchipss.cosmos.server.blocks;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;

/**
 * The instance of every block in the base game
 */
public class Blocks
{
	private static List<Block> allBlocks;
	
	public static final Block
		GRASS = new Block("grass"),
		DIRT  = new Block("dirt"),
		STONE = new Block("stone"),
		LIGHT = new Block("light"),
		LOG   = new Block("log"),
		LEAF  = new Block("leaf"),
		SHIP_CORE = new ShipCoreBlock(),
		SHIP_HULL = new Block("ship_hull"),
		SAND = new Block("sand"),
		SAND_STONE = new Block("sand_stone"),
		CACTUS = new Block("cactus");
	
	/**
	 * Adds all the blocks to a list
	 */
	public static void init()
	{
		allBlocks = Collections.unmodifiableList(
				Arrays.asList(STONE, GRASS, DIRT, 
						LIGHT, LOG, LEAF, SHIP_CORE, SHIP_HULL,
						SAND, SAND_STONE, CACTUS));
		
		for(short i = 0; i < allBlocks.size(); i++)
			allBlocks.get(i).blockId((short)(i + 1));
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
