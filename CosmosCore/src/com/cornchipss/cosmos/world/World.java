package com.cornchipss.cosmos.world;

import java.util.LinkedList;
import java.util.List;

import org.joml.Vector3f;
import org.joml.Vector3fc;

import com.cornchipss.cosmos.physx.RigidBody;
import com.cornchipss.cosmos.structures.Structure;

public class World
{
	private List<RigidBody> bodies;
	
	private List<Structure> structures;
	
	public World()
	{
		bodies = new LinkedList<>();
		structures = new LinkedList<>();
	}
	
	public void addRigidBody(RigidBody bdy)
	{
		bodies.add(bdy);
	}
	
	public void update(float delta)
	{
		Vector3f temp = new Vector3f();
		
		for(RigidBody b : bodies)
		{
			temp.set(b.velocity()).mul(delta);
			
			b.transform().position(temp.add(b.transform().position()));
		}
		
		for(Structure s : structures)
			s.update(delta);
	}

	public List<Structure> structures()
	{
		return structures;
	}
	
	/**
	 * TODO: this
	 * @param location The location to select nearby players for
	 * @return The structures that are near that location (not yet implemented - just returns them all)
	 */
	public List<Structure> structuresNear(Vector3fc location)
	{
		return structures;
	}

	public void addStructure(Structure s)
	{
		structures.add(s);
	}
}
