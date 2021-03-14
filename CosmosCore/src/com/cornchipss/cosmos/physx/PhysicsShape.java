package com.cornchipss.cosmos.physx;

import org.joml.Vector3fc;

import com.cornchipss.cosmos.blocks.BlockFace;

public interface PhysicsShape
{
	public BlockFace[] faces();
	public Vector3fc[] sides();
}
