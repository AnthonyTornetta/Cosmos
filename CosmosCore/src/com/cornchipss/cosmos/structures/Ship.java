package com.cornchipss.cosmos.structures;

import org.joml.Vector3f;
import org.joml.Vector3fc;

import com.cornchipss.cosmos.utils.Utils;
import com.cornchipss.cosmos.world.World;
import com.cornchipss.cosmos.world.entities.player.Player;

/**
 * A structure representing a ship
 */
public class Ship extends Structure
{
	private final static int MAX_DIMENSIONS = 16 * 10;
	
	private Player pilot;
	
	private Vector3f corePos = new Vector3f();
	
	public Ship(World world)
	{
		super(world, MAX_DIMENSIONS, MAX_DIMENSIONS, MAX_DIMENSIONS);
	}
	
	public Vector3fc corePosition()
	{
		corePos.set(MAX_DIMENSIONS / 2.f, MAX_DIMENSIONS / 2.f, MAX_DIMENSIONS / 2.f);
		return corePos;
	}
	
	int temp = 0;
	
	@Override
	public void update(float delta)
	{
		if(pilot == null)
			body().velocity(body().velocity().mul(0.99f)); // no more drifting into space once the pilot leaves
	}

	public void setPilot(Player p)
	{
		if(!Utils.equals(pilot, p))
		{
			if(pilot != null)
				pilot.shipPiloting(null);
			
			pilot = p;
			if(p != null)
				p.shipPiloting(this);
		}
	}
	
	public Player pilot()
	{
		return pilot;
	}
}
