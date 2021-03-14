package com.cornchipss.cosmos.blocks.data;

import java.util.HashMap;
import java.util.Map;

/**
 * <p>Data for a block in the world</p>
 * <p>Unique to each block</p>
 */
public class BlockData
{
	private Map<String, Object> data;

	/**
	 * <p>Data for a block in the world</p>
	 * <p>Unique to each block</p>
	 */
	public BlockData()
	{
		data = new HashMap<>();
	}
	
	/**
	 * The data at a given tag
	 * @param key the tag
	 * @return the data
	 */
	public Object data(String key)
	{
		return data.get(key);
	}
	
	/**
	 * Sets the data at a given tag
	 * @param key the tag
	 * @param value the data
	 */
	public void data(String key, Object value)
	{
		data.put(key, value);
	}
}
