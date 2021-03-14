package com.cornchipss.cosmos.lights;

import java.util.HashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;

import org.joml.Vector3i;
import org.joml.Vector3ic;

public class LightMap
{
	public static final float BLOCKED = -1;
	
	private Map<Vector3ic, LightSource> lightSources = new HashMap<>();
	
	private float[][][] lightMap;
	
	private boolean calculated = false;
	
	public LightMap(int w, int h, int l)
	{
		lightMap = new float[l][h][w];
	}
	
	public void addBlocking(int x, int y, int z)
	{
		lightMap[z][y][x] = BLOCKED;
	}
	
	public boolean removeBlocking(int x, int y, int z)
	{
		boolean blocked = lightMap[z][y][x] == BLOCKED;
		
		if(blocked)
			lightMap[z][y][x] = 0.0f;
		
		return blocked;
	}
	
	public boolean calculated() { return calculated; }
		
	/**
	 * Calculates the light map from scratch - overrides any previous light values
	 * Keeps anything in the light map marked as {@linkplain LightMap#BLOCKED}
	 * @return If this is calculated before, 
	 * returns the bounds of the parts of it that changed [ leftmost corner, rightmost corner ] - 
	 * otherwise the return result isn't of note, but it still has a size of 2 non-null values.  
	 * If no changes were made, they will both be (-1, -1, -1)
	 */
	public Vector3i[] calculateLightMap()
	{
		float[][][] oldMap = null;
		
		if(calculated)
		{
			// TODO: make this better
			
			oldMap = new float[lightMap.length][lightMap[0].length][lightMap[0][0].length];
			
			for(int z = 0; z < lightMap.length; z++)
			{
				for(int y = 0; y < lightMap[z].length; y++)
				{
					for(int x = 0; x < lightMap[z][y].length; x++)
					{
						oldMap[z][y][x] = lightMap[z][y][x];
						
						if(lightMap[z][y][x] != BLOCKED)
							lightMap[z][y][x] = 0;
					}
				}
			}
		}
		
		Vector3i extremeNeg = new Vector3i(-1), extremePos = new Vector3i(-1);
		
		for(Vector3ic originPos : lightSources.keySet())
		{
			LightSource src = lightSources.get(originPos);
			
			List<Vector3ic> positions = new LinkedList<>();
			
			float stren = 1.0f;
			
			positions.add(new Vector3i(
					originPos.x(), 
					originPos.y(), 
					originPos.z()));
			
			// makes it not BLOCKED so this progresses - will be overridden in first iteration of below code
			removeBlocking(originPos.x(), originPos.y(), originPos.z());
			
			while(positions.size() != 0 && stren > 0)
			{
				float nextStren = stren - 1.0f / src.strength();
				
				int oldSize = positions.size();
				
				while(oldSize != 0)
				{
					Vector3ic pos = positions.remove(0);
					oldSize--;
					
					int x = pos.x(), y = pos.y(), z = pos.z();
					
					if(at(x, y, z) < stren)
					{
						set(x, y, z, stren);
						
						if(nextStren > 0)
						{
							if(isGood(x + 1, y, z, nextStren, lightMap))//(isGood(x + 1, y, z, nextStren, lightMap))
								positions.add(new Vector3i(x + 1, y, z));
							if(isGood(x - 1, y, z, nextStren, lightMap))
								positions.add(new Vector3i(x - 1, y, z));
							if(isGood(x, y + 1, z, nextStren, lightMap))
								positions.add(new Vector3i(x, y + 1, z));
							if(isGood(x, y - 1, z, nextStren, lightMap))
								positions.add(new Vector3i(x, y - 1, z));
							if(isGood(x, y, z + 1, nextStren, lightMap))
								positions.add(new Vector3i(x, y, z + 1));
							if(isGood(x, y, z - 1, nextStren, lightMap))
								positions.add(new Vector3i(x, y, z - 1));
						}
					}
				}
				
				stren = nextStren;
			}
		}
		
		if(calculated)
		{
			for(int z = 0; z < lightMap.length; z++)
			{
				for(int y = 0; y < lightMap[z].length; y++)
				{
					for(int x = 0; x < lightMap[z][y].length; x++)
					{
						if(oldMap[z][y][x] != lightMap[z][y][x])
						{
							if(x < extremeNeg.x || extremeNeg.x == -1)
								extremeNeg.x = x;
							if(y < extremeNeg.y || extremeNeg.y == -1)
								extremeNeg.y = y;
							if(z < extremeNeg.z || extremeNeg.z == -1)
								extremeNeg.z = z;
							
							if(x > extremePos.x || extremePos.x == -1)
								extremePos.x = x;
							if(y > extremePos.y || extremePos.y == -1)
								extremePos.y = y;
							if(z > extremePos.z || extremePos.z == -1)
								extremePos.z = z;
						}
					}
				}
			}
		}
		
		calculated = true;
		
		return new Vector3i[] { extremeNeg, extremePos };
	}
	
	private boolean isGood(int x, int y, int z, float stren, float[][][] lightMap)
	{
		return within(x, y, z, lightMap) && at(x, y, z) != -1 && at(x, y, z) < stren;
	}
	
	public boolean within(int x, int y, int z, float[][][] lightMap)
	{
		return z >= 0 && z < lightMap.length &&
				y >= 0 && y < lightMap[z].length &&
				x >= 0 && x < lightMap[z][y].length;
	}
	
	public LightSource lightSource(int x, int y, int z, int offX, int offY, int offZ)
	{
		return lightSources.get(new Vector3i(x + offX, y + offY, z + offZ));
	}
	
	public LightSource lightSource(int x, int y, int z)
	{
		return lightSources.get(new Vector3i(x, y, z));
	}
	
	public void removeLightSource(int x, int y, int z)
	{
		lightSources.remove(new Vector3i(x, y, z));
	}

	public boolean hasLightSource(int x, int y, int z, int offX, int offY, int offZ)
	{
		return hasLightSource(x + offX, y + offY, z + offZ);
	}

	public boolean hasLightSource(int x, int y, int z)
	{
		return lightSource(x, y, z) != null;
	}
	
	public void lightSource(int x, int y, int z, LightSource src)
	{
		// TODO: check for existing source
		lightSources.put(new Vector3i(x, y, z), src);
	}
	
	public void lightSource(int x, int y, int z, int offX, int offY, int offZ, LightSource src)
	{
		lightSource(x + offX, y + offY, z + offZ, src);
	}
	
	public void set(int x, int y, int z, float f)
	{
		lightMap[z][y][x] = f;
	}
	
	public float at(int x, int y, int z)
	{
		return lightMap[z][y][x];
	}
	
	public float at(int x, int y, int z, int offX, int offY, int offZ)
	{
		return lightMap[z + offZ][y + offY][x + offX];
	}
	
	public boolean within(int x, int y, int z)
	{
		return z >= 0 && z < lightMap.length &&
				y >= 0 && y < lightMap[z].length &&
				x >= 0 && x < lightMap[z][y].length;
	}
	
	public boolean within(int x, int y, int z, int offX, int offY, int offZ)
	{
		return within(x + offX, y + offY, z + offZ);
	}
}
