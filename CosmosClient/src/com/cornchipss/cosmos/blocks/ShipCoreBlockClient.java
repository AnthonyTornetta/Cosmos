package com.cornchipss.cosmos.blocks;

import com.cornchipss.cosmos.blocks.data.BlockData;
import com.cornchipss.cosmos.models.ShipCoreModel;
import com.cornchipss.cosmos.structures.Ship;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.world.entities.player.Player;

/**
 * <p>The core block of any ship</p>
 * <p>If this is removed, then there is no ship</p>
 */
public class ShipCoreBlockClient extends ShipBlockClient implements IHasData, IInteractable
{
	/**
	 * <p>The core block of any ship</p>
	 * <p>If this is removed, then there is no ship</p>
	 */
	public ShipCoreBlockClient()
	{
		super(new ShipCoreModel(), "ship_core");
	}

	@Override
	public boolean canAddTo(Structure s)
	{
		return false; // The player cannot place this without creating a ship - where the block is automatically placed.
	}
	
	@Override
	public BlockData generateData(Structure s, int x, int y, int z)
	{
		BlockData data = new BlockData();
		data.data("ship", s);
		return data;
	}

	@Override
	public void onInteract(Structure s, Player p)
	{
		Ship ship = (Ship)s; // this will always be on a ship
		
		ship.setPilot(p);
	}
}
