package com.cornchipss.cosmos.blocks;

import com.cornchipss.cosmos.blocks.data.BlockData;
import com.cornchipss.cosmos.models.CubeModel;
import com.cornchipss.cosmos.models.IHasModel;

/**
 * <p>A block in the world</p>
 * <p>Only one instance of each block should ever be present</p>
 * <p>Each block of the same type in the world points to that instance</p>
 * <p>Use {@link IHasData} to differentiate between different blocks</p>
 */
public class ClientBlock extends Block implements IHasModel
{
	private CubeModel model;
	
	/**
	 * <p>A block in the world</p>
	 * <p>Only one instance of each block should ever be present</p>
	 * <p>Each block of the same type in the world points to that instance</p>
	 * <p>Use {@link BlockData} to differentiate between different blocks</p>
	 * 
	 * @param m The model the block has
	 * @param name The name used to refer to the block in the registry
	 */
	public ClientBlock(CubeModel m, String name)
	{
		super(name);
		
		this.model = m;
	}
	
	@Override
	public CubeModel model()
	{
		return model;
	}
}
