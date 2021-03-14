package com.cornchipss.cosmos.server.biospheres;

import org.joml.SimplexNoise;

import com.cornchipss.cosmos.blocks.Blocks;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.Maths;

/**
 * A test biosphere that just generates some basic terrain
 */
public class DesertBiosphere extends Biosphere
{
	@Override
	protected void generateTerrain(Structure s)
	{
		for(int z = 0; z < s.length(); z++)
		{
			for(int x = 0; x < s.width(); x++)
			{
				float n = 30 * SimplexNoise.noise(x * 0.01f, z * 0.01f);
				
				int h = Maths.round(s.height() - 30 + n);
				
				for(int y = 0; y < h; y++)
				{
					if(h - y < 5)
						s.block(x, y, z, Blocks.SAND);
					else
						s.block(x, y, z, Blocks.SAND_STONE);
				}
				
				if((x+8) % 8 == 0 && (z+8) % 8 == 0)
				{
					s.block(x, h, z, Blocks.LOG);
					s.block(x, h + 1, z, Blocks.LIGHT);
				}
			}
		}
	}

	@Override
	protected void populate(Structure s)
	{
		for(int z = 0; z < s.length(); z++)
		{
			for(int x = 0; x < s.width(); x++)
			{
				int y = s.higehstYAt(x, z);
				
				if(y != -1)
				{
					if(Math.random() < 0.01f)
					{
						if(s.block(x, y, z).equals(Blocks.SAND))
							generateCactus(x, y + 1, z, s);
					}
				}
			}
		}
	}
	
	/**
	 * Makes a happy cactus
	 * @param x x
	 * @param y y
	 * @param z z
	 * @param s the planet to do it on
	 */
	private void generateCactus(int x, int y, int z, Structure s)
	{
		int h = (int)(Math.random() * 4) - 1;
		for(int dy = 0; dy <= h; dy++)
			s.block(x, y + dy, z, Blocks.CACTUS);
	}
}
