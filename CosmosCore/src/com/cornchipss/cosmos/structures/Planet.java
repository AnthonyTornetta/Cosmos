package com.cornchipss.cosmos.structures;

import com.cornchipss.cosmos.world.World;

public class Planet extends Structure
{
	public Planet(World world, int width, int height, int length)
	{
		super(world, width, height, length);
	}
	
	public Planet(World world)
	{
		super(world);
	}
}
