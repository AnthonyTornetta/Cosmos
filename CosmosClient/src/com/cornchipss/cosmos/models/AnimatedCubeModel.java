package com.cornchipss.cosmos.models;

import com.cornchipss.cosmos.blocks.BlockFace;
import com.cornchipss.cosmos.material.Materials;

public abstract class AnimatedCubeModel extends CubeModel
{
	/**
	 * Used by the {@link Materials#ANIMATED_DEFAULT_MATERIAL}
	 * @param side The side of the block
	 * @return The number of animation frames this face has
	 */
	public abstract int maxAnimationStage(BlockFace side);
	
	public abstract float animationDelay(BlockFace side);
}
