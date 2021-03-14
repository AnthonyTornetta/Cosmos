package com.cornchipss.cosmos.physx;

import org.joml.Vector3f;
import org.joml.Vector3fc;

public class AABB
{
	private Vector3fc min, max;
	
	public static boolean within(Vector3f posA, AABB a, Vector3f posB, AABB b)
	{
		return posA.x + a.max.x() >= posB.x + b.min.x() && posA.x + a.min.x() <= posB.x + b.max.x() &&
				posA.y + a.max.y() >= posB.y + b.min.y() && posA.y + a.min.y() <= posB.y + b.max.y() &&
				posA.z + a.max.z() >= posB.z + b.min.z() && posA.z + a.min.z() <= posB.z + b.max.z();
	}
	
	public Vector3fc min() { return min; }
	public Vector3fc max() { return max; }
}
