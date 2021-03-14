package com.cornchipss.cosmos.world.entities.player;

import org.joml.Vector3f;
import org.joml.Vector3fc;

import com.cornchipss.cosmos.cameras.Camera;
import com.cornchipss.cosmos.inventory.Inventory;
import com.cornchipss.cosmos.physx.PhysicalObject;
import com.cornchipss.cosmos.physx.RayResult;
import com.cornchipss.cosmos.physx.RigidBody;
import com.cornchipss.cosmos.physx.Transform;
import com.cornchipss.cosmos.structures.Ship;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.Maths;
import com.cornchipss.cosmos.utils.Utils;
import com.cornchipss.cosmos.world.World;

public abstract class Player extends PhysicalObject
{
	private Ship pilotingShip;
	
	private Inventory inventory;
	private int selectedInventoryCol;
	
	public Player(World world)
	{
		super(world);
		
		inventory = new Inventory(4, 10);
	}
	
	public abstract void update(float delta);

	@Override
	public void addToWorld(Transform transform)
	{
		body(new RigidBody(transform));
		world().addRigidBody(body());
	}
	
	public Structure calculateLookingAt()
	{
		Vector3fc from = camera().position();
		Vector3f dLook = Maths.mul(camera().forward(), 50.0f);
		Vector3f to = Maths.add(from, dLook);
		
		Structure closestHit = null;
		float closestDistSqrd = -1;
		
		for(Structure s : world().structuresNear(body().transform().position()))
		{
			RayResult hits = s.shape().raycast(from, to);
			if(hits.closestHit() != null)
			{
				float distSqrd = Maths.distSqrd(from, hits.closestHitWorldCoords());
				
				if(closestHit == null)
				{
					closestHit = s;
					closestDistSqrd = distSqrd;
				}
				else if(closestDistSqrd > distSqrd)
				{
					closestHit = s;
					closestDistSqrd = distSqrd;
				}
			}
		}
		
		return closestHit;
	}

	public boolean isPilotingShip()
	{
		return pilotingShip != null;
	}
	
	public void shipPiloting(Ship s)
	{
		if(Utils.equals(pilotingShip, s))
			return;
		
		Ship temp = pilotingShip;
		pilotingShip = null;
		
		if(temp != null)
			temp.setPilot(null);
		
		if(s != null)
		{
			pilotingShip = s;
			pilotingShip.setPilot(this);
		}
	}
	
	public Ship shipPiloting()
	{
		return pilotingShip;
	}
	
	public abstract Camera camera();
	
	public void selectedInventoryColumn(int c)
	{
		selectedInventoryCol = c;
	}
	
	public int selectedInventoryColumn()
	{
		return selectedInventoryCol;
	}
	
	public Inventory inventory() { return inventory; }
	public void inventory(Inventory i) { inventory = i; }
}
