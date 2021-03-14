package com.cornchipss.cosmos.physx;

import java.util.LinkedList;
import java.util.List;

import org.joml.Intersectionf;
import org.joml.Vector3f;
import org.joml.Vector3fc;
import org.joml.Vector3i;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.blocks.BlockFace;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.Maths;

public class StructureShape
{
	private Structure s;
	
	public StructureShape(Structure s)
	{
		this.s = s;
	}
	
	/**
	 * Sends a raycast from a given point A to point B relative to world coordinates.
	 * @param from Point A
	 * @param to Point B
	 * @return A RayResult where the first point is marked as the closest.
	 */
	public RayResult raycast(Vector3fc from, Vector3fc to)
	{
		Vector3f intersectionPoint = new Vector3f();
		
		// Used  for checking the triangles of each cube face
		Vector3f temp1 = new Vector3f(), temp2 = new Vector3f(), temp3 = new Vector3f();
		
		List<Vector3f> hitPositions = new LinkedList<>();
		List<Vector3f> hits = new LinkedList<>();
		List<BlockFace> faces = new LinkedList<>();
		
		Vector3i coordsNeg = s.worldCoordsToStructureCoords(from);
		Vector3i coordsPos = s.worldCoordsToStructureCoords(to);
		
		if(coordsNeg.x > coordsPos.x)
		{
			int temp = coordsNeg.x;
			coordsNeg.x = coordsPos.x;
			coordsPos.x = temp;
		}
		
		if(coordsNeg.y > coordsPos.y)
		{
			int temp = coordsNeg.y;
			coordsNeg.y = coordsPos.y;
			coordsPos.y = temp;
		}
		
		if(coordsNeg.z > coordsPos.z)
		{
			int temp = coordsNeg.z;
			coordsNeg.z = coordsPos.z;
			coordsPos.z = temp;
		}
		
		for(int z = Maths.max(coordsNeg.z, 0); z <= Maths.min(s.length() - 1, coordsPos.z); z++)
		{
			for(int y = Maths.max(coordsNeg.y, 0); y <= Maths.min(s.height() - 1, coordsPos.y); y++)
			{
				for(int x = Maths.max(coordsNeg.x, 0); x <= Maths.min(s.width() - 1, coordsPos.x); x++)
				{
					Block b = s.block(x, y, z);
					
					if(b != null)
					{
						PhysicsShape sh = new CubeShape();
						
						for(int i = 0; i < sh.sides().length; i+=3)
						{
							BlockFace face = sh.faces()[i/3];
							
							temp1.set(x + sh.sides()[i].x(), y + sh.sides()[i].y(), z + sh.sides()[i].z());
							temp2.set(x + sh.sides()[i+1].x(), y + sh.sides()[i+1].y(), z + sh.sides()[i+1].z());
							temp3.set(x + sh.sides()[i+2].x(), y + sh.sides()[i+2].y(), z + sh.sides()[i+2].z());
							
							s.localCoordsToWorldCoords(temp1, temp1);
							s.localCoordsToWorldCoords(temp2, temp2);
							s.localCoordsToWorldCoords(temp3, temp3);
							
							if(Intersectionf.intersectLineSegmentTriangle(from, to, 
									temp1, temp2, temp3, 
									(float) 1E-9, intersectionPoint))
							{
								faces.add(face);
								hits.add(new Vector3f(x, y, z));
								hitPositions.add(new Vector3f(intersectionPoint));
							}
						}
					}
				}
			}
		}
		
		RayResult res = new RayResult(from, to, s, hits, faces);
		
		return res;
	}
}
