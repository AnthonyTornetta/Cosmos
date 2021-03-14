package com.cornchipss.cosmos.structures;

import java.io.DataInputStream;
import java.io.IOException;
import java.util.LinkedHashSet;
import java.util.Set;

import org.joml.Vector3i;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.lights.LightMap;
import com.cornchipss.cosmos.utils.Logger;
import com.cornchipss.cosmos.utils.Maths;
import com.cornchipss.cosmos.world.Chunk;
import com.cornchipss.cosmos.world.World;

public abstract class ClientStructure extends Structure
{
	private LightMap lightMap;
	
	private Set<Chunk> bulkUpdate;

	public ClientStructure(World world)
	{
		super(world);
	}
	
	public ClientStructure(World world, int width, int height, int length)
	{
		super(world, width, height, length);
		
		lightMap = new LightMap(width + 2, height + 2, length + 2);
	}
	
	@Override
	protected void setblockAt(int x, int y, int z, Block b)
	{
		Chunk c = chunk(x, y, z);
		
		c.block(x % Chunk.WIDTH, y % Chunk.HEIGHT, z % Chunk.LENGTH, b, !bulkUpdating());
		
		if(bulkUpdating())
			bulkUpdate.add(c);
	}
	
	@Override
	public void read(DataInputStream reader) throws IOException
	{
		super.read(reader);
		
		lightMap = new LightMap(width() + 2, height() + 2, length() + 2);
	}
	
	public void explode(int radius, Vector3i pos)
	{
		beginBulkUpdate();
		super.explode(radius, pos);
		endBulkUpdate();
	}

	public boolean bulkUpdating()
	{
		return bulkUpdate != null;
	}

	public void beginBulkUpdate()
	{
		if(!bulkUpdating())
			bulkUpdate = new LinkedHashSet<>();
	}

	public void endBulkUpdate()
	{
		if(bulkUpdating())
		{
			calculateLights(true);
			
			Set<Chunk> all = new LinkedHashSet<>();
		
			for(Chunk c : bulkUpdate)
			{
				// not the best way for account for all changes, given that large light sources would invalidate this, but it works for now
				all.add(c);
				if(c.leftNeighbor() != null)
					all.add(c.leftNeighbor());
				if(c.rightNeighbor() != null)
					all.add(c.rightNeighbor());
				if(c.topNeighbor() != null)
					all.add(c.topNeighbor());
				if(c.bottomNeighbor() != null)
					all.add(c.bottomNeighbor());
				if(c.frontNeighbor() != null)
					all.add(c.frontNeighbor());
				if(c.backNeighbor() != null)
					all.add(c.backNeighbor());
			}
			
			for(Chunk c : all)
				c.render();
		}
		else
			throw new IllegalStateException("Cannot end a bulk update when there is no bulk update currently happening");
		
		bulkUpdate = null;
	}
	
	public void calculateLights(boolean render)
	{
		long start = System.currentTimeMillis();
		
		Vector3i[] changedArea = lightMap.calculateLightMap();
		
		long end = System.currentTimeMillis();
		
		Logger.LOGGER.debug(end - start + "ms to calculate light map");
		
		if(render)
		{
			Vector3i extremeNeg = changedArea[0];
			Vector3i extremePos = changedArea[1];
			
			if(extremeNeg.x() != -1) // if it isn't -1, then none of them are negative 1
			{
				// TODO: fix this, for some reason the extremeNeg + Pos calcs don't work. Idk why
				extremeNeg.x = Maths.min(extremeNeg.x - Chunk.WIDTH, 0);
				extremeNeg.y = Maths.min(extremeNeg.y - Chunk.HEIGHT, 0);
				extremeNeg.z = Maths.min(extremeNeg.z - Chunk.LENGTH, 0);
				
				extremePos.x = Maths.min(extremePos.x + Chunk.WIDTH, width());
				extremePos.y = Maths.min(extremePos.y + Chunk.HEIGHT, height());
				extremePos.z = Maths.min(extremePos.z + Chunk.LENGTH, length());
				
				// Account for the +2 size of the light map
				extremeNeg.x += 1;
				extremeNeg.y += 1;
				extremeNeg.z += 1;
				
				extremePos.x -= 1;
				extremePos.y -= 1;
				extremePos.z -= 1;
				
				for(int cz = extremeNeg.z() / 16; cz < Math.ceil(extremePos.z() / 16.0f); cz++)
				{
					for(int cy = extremeNeg.y() / 16; cy < Math.ceil(extremePos.y() / 16.0f); cy++)
					{
						for(int cx = extremeNeg.x() / 16; cx < Math.ceil(extremePos.x() / 16.0f); cx++)
						{
							chunk(cx, cy, cz).render();
						}
					}
				}
			}
		}
	}
	
	public LightMap lightMap()
	{
		return lightMap;
	}
}
