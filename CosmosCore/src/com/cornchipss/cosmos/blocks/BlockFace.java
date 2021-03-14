package com.cornchipss.cosmos.blocks;

import org.joml.Vector3f;

/**
 * The faces of each block
 */
public enum BlockFace
{
	FRONT(0), 
	BACK(1),
	TOP(2),
	BOTTOM(3),
	RIGHT(4),
	LEFT(5),
	MAX(5),
	UNKNOWN(-1);
	
	private int f;
	
	private BlockFace(int f)
	{
		this.f = f;
	}
	
	/**
	 * <ul>
	 * 	<li>0: FRONT</li>
	 * 	<li>1: FRONT</li>
	 * 	<li>2: FRONT</li>
	 * 	<li>3: FRONT</li>
	 * 	<li>4: FRONT</li>
	 * 	<li>5: FRONT</li>
	 * 	<li>-1: UNKNOWN</li>
	 * </ul>
	 */
	public int getValue()
	{
		return f;
	}

	/**
	 * <p>The block face that corresponds to that index</p>
	 * <ul>
	 * 	<li>0: FRONT</li>
	 * 	<li>1: FRONT</li>
	 * 	<li>2: FRONT</li>
	 * 	<li>3: FRONT</li>
	 * 	<li>4: FRONT</li>
	 * 	<li>5: FRONT</li>
	 * 	<li>?: UNKNOWN</li>
	 * </ul>
	 * @param i The index
	 * @return The block face that corresponds to that index
	 */
	public static BlockFace fromFaceIndex(int i)
	{
		switch(i)
		{
		case 0:
			return FRONT;
		case 1:
			return BACK;
		case 2:
			return TOP;
		case 3:
			return BOTTOM;
		case 4:
			return RIGHT;
		case 5:
			return LEFT;
		default:
			return UNKNOWN;
		}
	}
	
	/**
	 * The position relative to the block's center assuming no rotation
	 * @return The position relative to the block's center assuming no rotation
	 */
	public Vector3f getRelativePosition()
	{
		switch(this)
		{
		case FRONT:
			return new Vector3f(0, 0, .5f);
		case BACK:
			return new Vector3f(0, 0, -.5f);
		case TOP:
			return new Vector3f(0, .5f, 0);
		case BOTTOM:
			return new Vector3f(0, -.5f, 0);
		case RIGHT:
			return new Vector3f(.5f, 0, 0);
		case MAX:
		case LEFT:
			return new Vector3f(-.5f, 0, 0);
		default:
			return new Vector3f(0, 0, 0);
		}
	}
}
