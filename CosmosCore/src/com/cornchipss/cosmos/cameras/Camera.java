package com.cornchipss.cosmos.cameras;

import org.joml.Matrix4fc;
import org.joml.Vector3fc;

/**
 * Contains methods to set the view OpenGL view matrix
 */
public abstract class Camera
{
	/**
	 * The matrix passed into the shader
	 */
	public abstract Matrix4fc viewMatrix();
	
	/**
	 * The forward direction in respect to the camera
	 * @return The forward direction in respect to the camera
	 */
	public abstract Vector3fc forward();
	
	/**
	 * The right direction in respect to the camera
	 * @return The right direction in respect to the camera
	 */
	public abstract Vector3fc right();
	
	/**
	 * The upward direction in respect to the camera
	 * @return The upward direction in respect to the camera
	 */
	public abstract Vector3fc up();

	/**
	 * The camera's position
	 * @return The camera's position
	 */
	public abstract Vector3fc position();

	/**
	 * Sets the camera's rotation to zero
	 */
	public abstract void zeroRotation();
	
	/**
	 * Updates all the values
	 */
	public abstract void update();
}
