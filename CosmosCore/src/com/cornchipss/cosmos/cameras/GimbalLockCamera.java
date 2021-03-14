package com.cornchipss.cosmos.cameras;

import org.joml.Matrix4f;
import org.joml.Matrix4fc;
import org.joml.Vector3f;
import org.joml.Vector3fc;

import com.cornchipss.cosmos.physx.PhysicalObject;
import com.cornchipss.cosmos.utils.Maths;

/**
 * <p>A camera that treats every rotation as an absolute rotation.  This also suffers from the gimbal lock most first person cameras suffer from.</p>
 * <p>See <a href="https://en.wikipedia.org/wiki/Gimbal_lock">https://en.wikipedia.org/wiki/Gimbal_lock</a></p>
 */
public class GimbalLockCamera extends Camera
{
	private Matrix4f matrix;
	private PhysicalObject parent;
	
	private Vector3f forward, right, up;
	private Vector3f rot;
	
	/**
	 * <p>A camera that treats every rotation as an absolute rotation.  This also suffers from the gimbal lock most first person cameras suffer from.</p>
	 * <p>See <a href="https://en.wikipedia.org/wiki/Gimbal_lock">https://en.wikipedia.org/wiki/Gimbal_lock</a></p>
	 * @param parent The parent this camera sits on
	 */
	public GimbalLockCamera(PhysicalObject parent)
	{
		this.parent = parent;
		
		rot = new Vector3f();
		matrix = new Matrix4f();
		
		forward = new Vector3f(0, 0, -1); // opengl moment
		right = new Vector3f(1, 0, 0);
		up = new Vector3f(0, 1, 0);
		
		update();
	}
	
	/**
	 * Call this after every update to a variable - updates all the other variables
	 */
	public void update()
	{
		if(parent.initialized())
		{
			rot.x = Maths.clamp(rot.x(), -Maths.PI / 2, Maths.PI / 2);
			
			Maths.createViewMatrix(position(), new Vector3f(rot), matrix);
			
			forward.x = Maths.sin(rot.y()) * Maths.cos(rot.x());
		    forward.y = Maths.sin(-rot.x());
		    forward.z = -Maths.cos(rot.x()) * Maths.cos(rot.y());
		    
		    right.x = Maths.cos(rot.y());
		    right.z = Maths.sin(rot.y());
		    
		    up.x = Maths.sin(rot.y()) * Maths.sin(rot.x());
		    up.y = Maths.cos(rot.x());
		    up.z = -Maths.sin(rot.x()) * Maths.cos(rot.y());
		}
	}
	
	/**
	 * Rotates the camera (assumes they are absolute rotations)
	 * @param delta The amount to rotate it by
	 */
	public void rotate(Vector3fc delta)
	{
		rot.add(delta);
	}
	
	/**
	 * Sets the camera's absolute rotation
	 * @param r The absolute rotation
	 */
	public void rotation(Vector3fc r)
	{
		rot.set(r);
	}
	
	@Override
	public Matrix4fc viewMatrix()
	{
		return matrix;
	}

	@Override
	public Vector3fc forward()
	{
		return forward;
	}

	@Override
	public Vector3fc right()
	{
		return right;
	}

	@Override
	public Vector3fc up()
	{
		return up;
	}

	/**
	 * Sets the camera's parent
	 * @param transform The camera's new parent
	 */
	public void parent(PhysicalObject transform)
	{
		this.parent = transform;
	}
	
	/**
	 * The camera's parent
	 * @return The camera's parent
	 */
	public PhysicalObject parent()
	{
		return parent;
	}

	@Override
	public Vector3fc position()
	{
		return new Vector3f(parent.position()).add(new Vector3f(0, 0.4f, 0));
	}

	@Override
	public void zeroRotation()
	{
		rot.set(0, 0, 0);
	}
}
