package com.cornchipss.cosmos.physx;

import java.util.Iterator;
import java.util.List;

import org.joml.Vector3f;
import org.joml.Vector3fc;

import com.cornchipss.cosmos.blocks.BlockFace;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.Maths;

public class RayResult
{
	private List<Vector3f> hits;
	private List<BlockFace> faces;
	
	private Structure s;
	private Vector3fc from, to;
	
	private Vector3f closestHit;
	private BlockFace closestFace;
	
	public RayResult(Vector3fc from, Vector3fc to, Structure s, List<Vector3f> hits, List<BlockFace> faces)
	{
		this.from = from;
		this.to = to;
		this.s = s;
		this.hits = hits;
		this.faces = faces;
	}
	
	public List<BlockFace> facesHit()
	{
		return faces;
	}
	
	public List<Vector3f> positionsHit()
	{
		return hits;
	}
	
	public int hits()
	{
		return hits.size();
	}
	
	public Vector3f closestHitWorldCoords()
	{
		if(closestHit == null)
			calculateClosest();
		
		return s.localCoordsToWorldCoords(Maths.add(closestFace.getRelativePosition(), closestHit));
	}
	
	public Vector3f closestHit()
	{
		if(closestHit == null)
			calculateClosest();
		
		return closestHit;
	}
	
	public BlockFace closestFace()
	{
		if(closestFace == null)
			calculateClosest();
		
		return closestFace;
	}
	
	private void calculateClosest()
	{
		if(hits.size() == 0)
			return;
		
		Iterator<Vector3f> hitsItr = hits.iterator();
		Iterator<BlockFace> facesItr = faces.iterator();
		
		closestHit = hitsItr.next();
		closestFace = facesItr.next();
		Vector3f t = closestFace.getRelativePosition().add(s.localCoordsToWorldCoords(closestHit));
		
		float closest = Maths.distSqrd(from, t);
		
		while(hitsItr.hasNext())
		{
			Vector3f hit = hitsItr.next();
			
			BlockFace face = facesItr.next();
			
			t = face.getRelativePosition().add(s.localCoordsToWorldCoords(hit));
			
			float d = Maths.distSqrd(from, t);
			
			if(d < closest)
			{
				closest = d;
				closestHit = hit;
				closestFace = face;
			}
		}
	}
	
	public Structure structure() { return s; }

	public Vector3fc from() { return from; }

	public Vector3fc to() { return to; }
}
