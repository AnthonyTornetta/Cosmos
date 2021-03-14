package com.cornchipss.cosmos.blocks;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;

import com.cornchipss.cosmos.lights.LightSource;
import com.cornchipss.cosmos.models.CactusModel;
import com.cornchipss.cosmos.models.DirtModel;
import com.cornchipss.cosmos.models.GrassModel;
import com.cornchipss.cosmos.models.LeafModel;
import com.cornchipss.cosmos.models.LightModel;
import com.cornchipss.cosmos.models.LogModel;
import com.cornchipss.cosmos.models.SandModel;
import com.cornchipss.cosmos.models.SandStoneModel;
import com.cornchipss.cosmos.models.ShipHullModel;
import com.cornchipss.cosmos.models.StoneModel;

/**
 * The instance of every block in the base game
 */
public class BlockList
{
	private static List<ClientBlock> allBlocks;
	
	public static final ClientBlock
		GRASS = new ClientBlock(new GrassModel(), "grass"),
		DIRT  = new ClientBlock(new DirtModel(), "dirt"),
		STONE = new ClientBlock(new StoneModel(), "stone"),
		LIGHT = new LitBlock(new LightModel(), new LightSource(16), "light"),
		LOG   = new ClientBlock(new LogModel(), "log"),
		LEAF  = new ClientBlock(new LeafModel(), "leaf"),
		SHIP_CORE = new ShipCoreBlockClient(),
		SHIP_HULL = new ClientBlock(new ShipHullModel(), "ship_hull"),
		SAND = new ClientBlock(new SandModel(), "sand"),
		SAND_STONE = new ClientBlock(new SandStoneModel(), "sand_stone"),
		CACTUS = new ClientBlock(new CactusModel(), "cactus");
	
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
	
	public static ClientBlock fromNumericId(short id)
	{
		if(id == 0)
			return null;
		return allBlocks.get(id - 1);
	}
	
	/**
	 * Returns a list of all the blocks in the game - this cannot be modified.
	 * @return a list of all the blocks in the game - this cannot be modified.
	 */
	public static List<ClientBlock> all()
	{
		return allBlocks;
	}
}
