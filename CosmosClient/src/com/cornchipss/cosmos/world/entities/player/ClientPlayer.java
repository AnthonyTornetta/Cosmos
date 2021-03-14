package com.cornchipss.cosmos.world.entities.player;

import org.joml.Vector3f;
import org.joml.Vector3fc;
import org.joml.Vector3i;
import org.lwjgl.glfw.GLFW;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.blocks.BlockFace;
import com.cornchipss.cosmos.blocks.Blocks;
import com.cornchipss.cosmos.blocks.IInteractable;
import com.cornchipss.cosmos.cameras.Camera;
import com.cornchipss.cosmos.cameras.GimbalLockCamera;
import com.cornchipss.cosmos.physx.RayResult;
import com.cornchipss.cosmos.physx.RigidBody;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.Maths;
import com.cornchipss.cosmos.utils.io.Input;
import com.cornchipss.cosmos.world.World;

public class ClientPlayer extends Player
{
	private GimbalLockCamera cam;
	
	public ClientPlayer(World world)
	{
		super(world);
		
		cam = new GimbalLockCamera(this);
		
		for(int i = 0; i < Blocks.all().size() && i < inventory().rows() * inventory().columns(); i++)
		{
			inventory().block(i / inventory().columns(), i % inventory().columns(), Blocks.all().get(i));
		}
	}
	
	@Override
	public void update(float delta)
	{
		if(!isPilotingShip())
		{
			handleHotbar();
			
			handleMovement(delta);
			
			handleInteractions();
		}
		else if(Input.isKeyJustDown(GLFW.GLFW_KEY_R))
			shipPiloting(null);
		
		camera().update();
	}
	
	private void handleInteractions()
	{
		Structure lookingAt = calculateLookingAt();
		
		if(lookingAt != null && (Input.isKeyJustDown(GLFW.GLFW_KEY_R) || 
						Input.isMouseBtnJustDown(GLFW.GLFW_MOUSE_BUTTON_1) || 
						Input.isMouseBtnJustDown(GLFW.GLFW_MOUSE_BUTTON_2) || 
						Input.isMouseBtnJustDown(GLFW.GLFW_MOUSE_BUTTON_3)))
		{
			Vector3fc from = camera().position();
			Vector3f dLook = Maths.mul(camera().forward(), 10.0f);
			Vector3f to = Maths.add(from, dLook);

			RayResult hits = lookingAt.shape().raycast(from, to);
			
			if(hits.closestHit() != null)
			{
				Block selectedBlock = null;
				
				selectedBlock = inventory().block(0, selectedInventoryColumn());
				
				Vector3i pos = new Vector3i(Maths.round(hits.closestHit().x()), 
						Maths.round(hits.closestHit().y()), 
						Maths.round(hits.closestHit().z()));
				
				if(Input.isMouseBtnJustDown(GLFW.GLFW_MOUSE_BUTTON_1))
				{
					lookingAt.block(pos.x, pos.y, pos.z, null);
				}
				else if(Input.isMouseBtnJustDown(GLFW.GLFW_MOUSE_BUTTON_2))
				{
					if(selectedBlock != null && selectedBlock.canAddTo(lookingAt))
					{
						BlockFace face = hits.closestFace();
						
						int xx = Maths.floor(pos.x + 0.5f + (face.getRelativePosition().x * 2)), 
							yy = Maths.floor(pos.y + 0.5f + (face.getRelativePosition().y * 2)), 
							zz = Maths.floor(pos.z + 0.5f + (face.getRelativePosition().z * 2));
						
						if(lookingAt.withinBlocks(xx, yy, zz) && !lookingAt.hasBlock(xx, yy, zz))
						{
							lookingAt.block(xx, yy, zz, selectedBlock);
						}
					}
				}
				else if(Input.isKeyJustDown(GLFW.GLFW_KEY_R))
				{
					if(lookingAt.block(pos.x, pos.y, pos.z) instanceof IInteractable)
					{
						((IInteractable)lookingAt.block(pos.x, pos.y, pos.z))
							.onInteract(lookingAt, this);
					}
				}
				else if(Input.isMouseBtnJustDown(GLFW.GLFW_MOUSE_BUTTON_3))
				{
					lookingAt.explode(20, pos);
				}
			}
		}
	}
	
	private void handleMovement(float delta)
	{
		Vector3f dVel = new Vector3f();
	    
		if(Input.isKeyDown(GLFW.GLFW_KEY_W))
			dVel.add(camera().forward());
		if(Input.isKeyDown(GLFW.GLFW_KEY_S))
			dVel.sub(camera().forward());
		if(Input.isKeyDown(GLFW.GLFW_KEY_D))
			dVel.add(camera().right());
		if(Input.isKeyDown(GLFW.GLFW_KEY_A))
			dVel.sub(camera().right());
		if(Input.isKeyDown(GLFW.GLFW_KEY_E))
			dVel.add(camera().up());
		if(Input.isKeyDown(GLFW.GLFW_KEY_Q))
			dVel.sub(camera().up());
		
		dVel.x = (dVel.x() * (delta * 1000));
		dVel.z = (dVel.z() * (delta * 1000));
		dVel.y = (dVel.y() * (delta * 1000));
		
		if(Input.isKeyDown(GLFW.GLFW_KEY_LEFT_CONTROL))
			dVel.mul(0.001f);
		
		Vector3f dRot = new Vector3f();
		
		dRot.y = (dRot.y() - Input.getMouseDeltaX() * 0.1f);
		
		dRot.x = (dRot.x() - Input.getMouseDeltaY() * 0.1f);
		
		dRot.mul(delta);
		
		cam.rotate(dRot);
		
		Vector3f vel = body().velocity();
		
		if(Input.isKeyDown(GLFW.GLFW_KEY_LEFT_SHIFT))
			vel.mul(0.75f);

		vel.add(dVel);

		vel = Maths.safeNormalize(vel, 50.0f);
		
		if(Input.isKeyJustDown(GLFW.GLFW_KEY_SPACE))
			vel.y = (vel.y() + 5);
		
		body().velocity(vel);
	}

	private void handleHotbar()
	{
		if(Input.isKeyJustDown(GLFW.GLFW_KEY_0))
		{
			selectedInventoryColumn(9);
		}
		else
		{
			for(int key = GLFW.GLFW_KEY_1; key <= GLFW.GLFW_KEY_9 + 1; key++)
			{
				if(Input.isKeyJustDown(key))
				{
					selectedInventoryColumn(key - GLFW.GLFW_KEY_1);
					break;
				}
			}
		}
	}
	
	@Override
	public Camera camera()
	{
		return cam;
	}
	
	@Override
	public void body(RigidBody b)
	{
		super.body(b);
		
		cam.parent(this);
	}
}
