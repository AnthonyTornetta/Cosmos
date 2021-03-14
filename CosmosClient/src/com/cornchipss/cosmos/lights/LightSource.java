package com.cornchipss.cosmos.lights;

import org.joml.Vector3f;
import org.joml.Vector3fc;

public class LightSource
{
	private final int dist;
	
	/**
	 * Not yet implemented
	 */
	private Vector3fc color;
	
	public LightSource(int dist)
	{
		this(dist, 1, 1, 1);
	}
	
	protected LightSource(int dist, float r, float g, float b)
	{
		this.dist = dist;
		
		this.color = new Vector3f(r, g, b);
	}
	
	public int strength() { return dist; }
	
	public Vector3fc color() { return color; }
	
	public float r() { return color.x(); }
	public float g() { return color.y(); }
	public float b() { return color.z(); }
}
