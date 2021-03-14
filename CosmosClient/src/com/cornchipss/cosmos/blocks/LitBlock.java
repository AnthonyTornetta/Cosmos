package com.cornchipss.cosmos.blocks;

import javax.annotation.Nonnull;

import com.cornchipss.cosmos.lights.LightSource;
import com.cornchipss.cosmos.models.CubeModel;

/**
 * A block that emits light
 */
public class LitBlock extends ClientBlock
{
	private LightSource source;
	
	/**
	 * A block that emits light
	 * @param m The model to use
	 * @param src The {@link LightSource} the block emits
	 */
	public LitBlock(CubeModel m, @Nonnull LightSource src, String name)
	{
		super(m, name);
		
		source = src;
	}
	
	public LightSource lightSource()
	{
		return source;
	}
}
