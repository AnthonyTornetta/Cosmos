package com.cornchipss.cosmos.registry;

import java.lang.reflect.Constructor;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import javax.annotation.Nonnull;

import com.cornchipss.cosmos.biospheres.Biosphere;

public class Biospheres
{
	private static Map<String, Class<? extends Biosphere>> biospheres = new HashMap<>();
	private static Map<String, Constructor<? extends Biosphere>> constructors = new HashMap<>();
	
	@SuppressWarnings("unchecked")
	public static void registerBiosphere(@Nonnull Class<? extends Biosphere> clazz, @Nonnull String id)
	{
		Constructor<?>[] ctors = clazz.getConstructors();
		
		boolean noarg = false;
		
		for(int i = 0; i < ctors.length; i++)
		{
			if(ctors[i].getParameterCount() == 0)
			{
				constructors.put(id, (Constructor<? extends Biosphere>) ctors[i]);
				noarg = true;
				break;
			}
		}
		
		if(!noarg)
			throw new IllegalArgumentException("The Biosphere must have a no-arg constructor!");
		
		biospheres.put(id, clazz);
	}
	
	public int getBiosphereAmount() { return biospheres.size(); }
	public static List<String> getBiosphereIds() { return new ArrayList<>(biospheres.keySet()); }
	public static Biosphere newInstance(String id)
	{
		try
		{
			return constructors.get(id).newInstance();
		}
		catch (Exception e)
		{
			throw new RuntimeException(e);
		}
	}
}