package com.cornchipss.cosmos.world;

import org.joml.Vector3fc;

import com.cornchipss.cosmos.structures.Structure;

public class Location
{
	private Vector3fc position;
	private Structure struct;
	
	public Location(Vector3fc position, Structure struct)
	{
		this.position = position;
		this.struct = struct;
	}
	
	public Vector3fc position()
	{
		return position;
	}
	
	public Structure structure()
	{
		return struct;
	}
}
