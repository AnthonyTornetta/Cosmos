package com.cornchipss.cosmos.physx;

import org.joml.Matrix4f;
import org.joml.Matrix4fc;
import org.joml.Quaternionf;
import org.joml.Quaternionfc;
import org.joml.Vector3f;
import org.joml.Vector3fc;
import org.joml.Vector4f;

import com.cornchipss.cosmos.utils.Maths;

public class Transform
{
	private Vector3f position;
	private Quaternionf rotation;
	
	private Matrix4f transMatrix;
	private Matrix4f invertedMatirx;
	
	private Vector3f forward, up, right;
	private Vector4f temp;

	public Transform()
	{
		this(0, 0, 0);
	}

	public Transform(float x, float y, float z)
	{
		position = new Vector3f(x, y, z);
		rotation = Maths.blankQuaternion();
		
		transMatrix = new Matrix4f();
		invertedMatirx = new Matrix4f();
		
		forward = new Vector3f(0, 0, 1);
		up = new Vector3f(0, 1, 0);
		right = new Vector3f(1, 0, 0);
		temp = new Vector4f();
		
		updateMatrix();
	}
	
	private void updateMatrix()
	{
		transMatrix.identity();
		
		transMatrix.rotate(rotation);

		transMatrix.transform(0, 0, -1, 1, temp); // opengl moment
		forward.set(temp.x, temp.y, temp.z);
		
		transMatrix.transform(1, 0, 0, 1, temp);
		right.set(temp.x, temp.y, temp.z);
		
		transMatrix.transform(0, 1, 0, 1, temp);
		up.set(temp.x, temp.y, temp.z);
		
		transMatrix.identity();
		
		transMatrix.translate(position);
		
		transMatrix.rotate(rotation);
		
		transMatrix.invert(invertedMatirx);
	}
	
	public Transform(Vector3fc pos)
	{
		this(pos.x(), pos.y(), pos.z());
	}
	
	public void position(Vector3fc p)
	{
		position.set(p);
		updateMatrix();
	}
	
	public Vector3fc position()
	{
		return position;
	}
	
	public Quaternionfc rotation()
	{
		return rotation;
	}
	
	public void rotation(Quaternionfc q)
	{
		rotation.set(q);
		updateMatrix();
	}

	public Matrix4fc matrix()
	{
		return transMatrix;
	}

	public Matrix4f invertedMatrix()
	{
		return invertedMatirx;
	}

	public Vector3fc forward()
	{
		return forward;
	}

	public Vector3fc right()
	{
		return right;
	}

	public Vector3fc up()
	{
		return up;
	}
}
