package com.cornchipss.cosmos.physx;

import org.joml.Vector3f;
import org.joml.Vector3fc;

import com.cornchipss.cosmos.blocks.BlockFace;

public class CubeShape implements PhysicsShape
{
	private static final BlockFace[] faces = new BlockFace[]
			{
					BlockFace.BACK,
					BlockFace.BACK,
					
					BlockFace.FRONT,
					BlockFace.FRONT,
					
					BlockFace.LEFT,
					BlockFace.LEFT,
					
					BlockFace.RIGHT,
					BlockFace.RIGHT,
					
					BlockFace.BOTTOM,
					BlockFace.BOTTOM,
					
					BlockFace.TOP,
					BlockFace.TOP,
			};
	
	private static final Vector3fc[] sides = new Vector3fc[]
			{
					// Back
					new Vector3f(0, 0, 0),
					new Vector3f(1, 0, 0),
					new Vector3f(1, 1, 0),
					
					new Vector3f(1, 1, 0),
					new Vector3f(0, 1, 0),
					new Vector3f(0, 0, 0),
					
					// Front
					new Vector3f(0, 0, 1),
					new Vector3f(1, 0, 1),
					new Vector3f(1, 1, 1),
					
					new Vector3f(1, 1, 1),
					new Vector3f(0, 1, 1),
					new Vector3f(0, 0, 1),
					
					// Left
					new Vector3f(0, 0, 0),
					new Vector3f(0, 1, 0),
					new Vector3f(0, 1, 1),
					
					new Vector3f(0, 1, 1),
					new Vector3f(0, 0, 1),
					new Vector3f(0, 0, 0),
					
					// Right
					new Vector3f(1, 0, 0),
					new Vector3f(1, 1, 0),
					new Vector3f(1, 1, 1),
					
					new Vector3f(1, 1, 1),
					new Vector3f(1, 0, 1),
					new Vector3f(1, 0, 0),
					
					// Bottom
					new Vector3f(0, 0, 0),
					new Vector3f(1, 0, 0),
					new Vector3f(1, 0, 1),
					
					new Vector3f(1, 0, 1),
					new Vector3f(0, 0, 1),
					new Vector3f(0, 0, 0),
					
					// Top
					new Vector3f(0, 1, 0),
					new Vector3f(1, 1, 0),
					new Vector3f(1, 1, 1),
					
					new Vector3f(1, 1, 1),
					new Vector3f(0, 1, 1),
					new Vector3f(0, 1, 0)
			};

	@Override
	public BlockFace[] faces()
	{
		return faces;
	}

	@Override
	public Vector3fc[] sides()
	{
		return sides;
	}

}
