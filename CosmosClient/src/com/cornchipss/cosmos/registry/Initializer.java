package com.cornchipss.cosmos.registry;

import com.cornchipss.cosmos.biospheres.DesertBiosphere;
import com.cornchipss.cosmos.biospheres.GrassBiosphere;
import com.cornchipss.cosmos.blocks.Blocks;
import com.cornchipss.cosmos.material.Materials;
import com.cornchipss.cosmos.utils.Logger;

public class Initializer
{
	public void init()
	{
		Logger.LOGGER.info("Initializing...");
		
		Blocks.init();
		
		Biospheres.registerBiosphere(GrassBiosphere.class, "cosmos:grass");
		Biospheres.registerBiosphere(DesertBiosphere.class, "cosmos:desert");

		Materials.initMaterials();
		
		Logger.LOGGER.info("Initialization Complete");
	}
}
