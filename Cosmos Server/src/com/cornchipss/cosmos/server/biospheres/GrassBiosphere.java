package com.cornchipss.cosmos.server.biospheres;

import org.joml.SimplexNoise;

import com.cornchipss.cosmos.biospheres.Biosphere;
import com.cornchipss.cosmos.blocks.Blocks;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.Maths;

/**
 * A test biosphere that just generates some basic terrain
 */
public class GrassBiosphere extends Biosphere
{
	@Override
	protected void generateTerrain(Structure s)
	{
		for(int z = 0; z < s.length(); z++)
		{
			for(int x = 0; x < s.width(); x++)
			{
				float n = 6 * SimplexNoise.noise(x * 0.03f, z * 0.03f);
				
				int REMOVE_ME_LATER = s.height() == 16 ? 8 : 16;
				int h = Maths.round(s.height() - REMOVE_ME_LATER + n);
				
				for(int y = 0; y < h; y++)
				{
					if(y == h - 1)
						s.block(x, y, z, Blocks.GRASS);
					else if(h - y < 5)
						s.block(x, y, z, Blocks.DIRT);
					else
						s.block(x, y, z, Blocks.STONE);
				}
				
				if((x+8) % 16 == 0 && (z+8) % 16 == 0)
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
						if(s.block(x, y, z).equals(Blocks.GRASS))
							generateTree(x, y + 1, z, s);
					}
				}
			}
		}
	}
	
	/**
	 * Makes a happy tree
	 * @param x x
	 * @param y y
	 * @param z z
	 * @param s the planet to do it on
	 */
	private void generateTree(int x, int y, int z, Structure s)
	{
		s.block(x, y, z, Blocks.LOG);
		s.block(x, y+1, z, Blocks.LOG);
		s.block(x, y+2, z, Blocks.LOG);
		
		for(int dy = 0; dy <= 4; dy++)
		{
			int off = dy <= 2 ? 2 : 1;
			
			for(int dz = -off; dz <= off; dz++)
			{
				for(int dx = -off; dx <= off; dx++)
				{
					if(s.withinBlocks(x+dx, y+3+dy, z+dz) &&
							!(dy == 4 && (dz == -1 || dz == 1) && (dx == -1 || dx == 1)))
					{
						if(dx == 0 && dz == 0 && dy != 4)
							s.block(x+dx, y+3+dy, z+dz, Blocks.LOG);
						else
							s.block(x+dx, y+3+dy, z+dz, Blocks.LEAF);
					}
				}
			}
		}
	}
}
