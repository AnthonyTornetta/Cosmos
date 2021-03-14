package com.cornchipss.cosmos.server.registry;

import com.cornchipss.cosmos.biospheres.DesertBiosphere;
import com.cornchipss.cosmos.biospheres.GrassBiosphere;
import com.cornchipss.cosmos.blocks.Blocks;
import com.cornchipss.cosmos.registry.Biospheres;
import com.cornchipss.cosmos.registry.Initializer;
import com.cornchipss.cosmos.utils.Logger;

public class ServerInitializer extends Initializer
{
	@Override
	public void init()
	{
		Logger.LOGGER.info("Initializing...");
		
		Blocks.init();
		
		Biospheres.registerBiosphere(GrassBiosphere.class, "cosmos:grass");
		Biospheres.registerBiosphere(DesertBiosphere.class, "cosmos:desert");
		
		Logger.LOGGER.info("Initialization Complete");
	}
}
