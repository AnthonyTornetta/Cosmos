package com.cornchipss.cosmos.utils;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.FloatBuffer;

import org.joml.Matrix4f;
import org.joml.Matrix4fc;
import org.joml.Quaternionf;
import org.joml.Quaternionfc;
import org.joml.Vector3f;
import org.joml.Vector3fc;
import org.joml.Vector3i;
import org.joml.Vector4f;
import org.joml.Vector4fc;

public class Maths
{
	/**
	 * A float of Math.PI
	 */
	public static final float PI = (float)Math.PI;
	
	/**
	 * Maths.PI * 2
	 */
	public static final float TAU = PI * 2;
	
	/**
	 * Maths.PI / 2
	 */
	public static final float PI2 = PI / 2;	
	
	/**
	 * How the equals function handles floats
	 */
	public static final float EQUALS_PRECISION = 0.0001f;
	
	/**
	 * Creates a view matrix based on coordinates + rotations
	 * @param x X
	 * @param y Y
	 * @param z Z
	 * @param rx Rotation X
	 * @param ry Rotation Y
	 * @param rz Rotation Z
	 * @param dest The destiantion matrix
	 */
	public static void createViewMatrix(float x, float y, float z, float rx, float ry, float rz, Matrix4f dest)
	{
		dest.identity();
		
		dest.rotate(rx, 1, 0, 0);
		dest.rotate(ry, 0, 1, 0);
		dest.rotate(rz, 0, 0, 1);
		
		dest.translate(-x, -y, -z);
	}
	
	public static void createViewMatrix(Vector3fc position, Quaternionfc rotation, Matrix4f dest)
	{
		dest.identity();
		
		dest.rotate(rotation);
		
		dest.translate(-position.x(), -position.y(), -position.z());
	}
	
	/**
	 * Creates a view matrix based on coordinates + rotations
	 * @param pos Position
	 * @param rot Rotation
	 * @param dest The destiantion matrix
	 */
	public static void createViewMatrix(Vector3fc pos, Vector3fc rot, Matrix4f dest)
	{
		createViewMatrix(pos.x(), pos.y(), pos.z(), rot.x(), rot.y(), rot.z(), dest);
	}
	
	public static Matrix4f createTransformationMatrix(Vector3fc position, float rx, float ry, float rz)
	{
		return createTransformationMatrix(position, rx, ry, rz, 1);
	}
	
	public static final Vector3fc 
		RIGHT = new Vector3f(1,0,0), 
		UP = new Vector3f(0,1,0), 
		FORWARD = new Vector3f(0,0,1);
	
	public static Matrix4f createTransformationMatrix(Vector3fc pos, float rx, float ry, float rz, float scale)
	{
        Matrix4f matrix = new Matrix4f();
        matrix.identity();
        matrix.translate(pos);
        matrix.rotate(rx, RIGHT);
        matrix.rotate(ry, UP);
        matrix.rotate(rz, FORWARD);
        matrix.scale(new Vector3f(scale, scale, scale));
        return matrix;
    }
	
	public static Matrix4f createTransformationMatrix(Vector3fc pos, Quaternionfc rot)
	{
		Matrix4f matrix = new Matrix4f();
        matrix.identity();
        matrix.rotate(rot);
        matrix.translate(pos);
        return matrix;
	}
	
	public static Matrix4f createRotationMatrix(Quaternionfc q)
	{
		FloatBuffer buf = ByteBuffer.allocate(4 * 16).order(ByteOrder.nativeOrder()).asFloatBuffer();
		q.getAsMatrix4f(buf);
		return new Matrix4f(buf);
	}
	
	public static Matrix4f createRotationMatrix(Vector3f axis, float angle)
	{
		return createRotationMatrix(axis, angle);
	}
	
	public static Matrix4f createRotationMatrix(Vector3fc axis, float angle)
	{
		/*
		 * https://open.gl/transformations
		 */
		
		if(axis.y() != 0)
			angle -= Maths.PI / 2; // OpenGL is funny for whatever reason
		
	    float s = sin(angle);
	    float c = cos(angle);
	    float oc = 1.0f - c;
	    
	    return new Matrix4f(oc * axis.x() * axis.x() + c,           oc * axis.x() * axis.y() - axis.z() * s,  oc * axis.z() * axis.x() + axis.y() * s,  0.0f,
	                oc * axis.x() * axis.y() + axis.z() * s,  oc * axis.y() * axis.y() + c,           oc * axis.y() * axis.z() - axis.x() * s,  0.0f,
	                oc * axis.z() * axis.x() - axis.y() * s,  oc * axis.y() * axis.z() + axis.x() * s,  oc * axis.z() * axis.z() + c,           0.0f,
	                0.0f,                                0.0f,                                0.0f,                                1.0f);
	}
	
	public static Matrix4f createCombinedRotationMatrix(Vector3fc rotation)
	{
		return createRotationMatrix(Utils.x(), rotation.x()).mul(createRotationMatrix(Utils.y(), rotation.y()).mul(createRotationMatrix(Utils.z(), rotation.z())));
	}
	
	@Deprecated
	/**
	 * idk if this works 
	 * @param pos
	 * @param rotations
	 * @return
	 */
	public static Vector3f getPositionActual(Vector3fc pos, Matrix4fc... rotations)
	{
		Matrix4f rotationFinal = new Matrix4f();
		rotationFinal.identity();
		
		for(Matrix4fc rot : rotations)
			rotationFinal.mul(rot);
		
		Vector4f vec = new Vector4f(pos.x(), pos.y(), pos.z(), 0).mul(rotationFinal);
		
		return new Vector3f(vec.x, vec.y, vec.z);
	}
	
	public static float cos(float theta)
	{
		return (float)Math.cos(theta);
	}
	
	public static float sin(float theta)
	{
		return (float)Math.sin(theta);
	}
	
	public static float tan(float theta)
	{
		return (float)Math.tan(theta);
	}
	
	public static Quaternionf blankQuaternion()
	{
		return new Quaternionf(0, 0, 0, 1);
	}
	
	public static Quaternionf quaternionFromRotation(float z, float y, float x)
	{
		float sx = Maths.sin(x / 2);
		float cx = Maths.cos(x / 2);
		float sy = Maths.sin(y / 2);
		float cy = Maths.cos(y / 2);
		float sz = Maths.sin(z / 2);
		float cz = Maths.cos(z / 2);
		
		float qx = sz * cy * cx - cz * sy * sx;
		float qy = cz * sy * cx + sz * cy * sx;
		float qz = cz * cy * sx - sz * sy * cx;
		float qw = cz * cy * cx + sz * sy * sx;
		
		return new Quaternionf(qx, qy, qz, qw);
	}
	
	public static Quaternionf quaternionFromRotation(Vector3fc rot)
	{
		return quaternionFromRotation(rot.x(), rot.y(), rot.z());
	}
	
	public static Vector3f rotatePoint(Vector3fc point, Vector3fc rotation)
	{
		Quaternionf transQuat = blankQuaternion();
		
		Vector3f punto = new Vector3f(point.x(), point.y(), point.z());
		rotation = mod(rotation, Maths.TAU);
		
		Quaternionf rotationQuat = blankQuaternion();
		
		rotationQuat.rotateXYZ(rotation.x(), rotation.y(), rotation.z(), transQuat);
		
		transQuat.transform(punto);
		
		return punto;
	}
	
	public static Vector3f rotatePoint(Matrix4fc rotationMatrixX, Matrix4fc rotationMatrixY, Matrix4fc rotationMatrixZ, Vector3f point)
	{
		return rotatePoint(rotationMatrixX, rotationMatrixX, rotationMatrixX, new Vector4f(point.x(), point.y(), point.z(), 0));
	}
	
	public static Vector3f rotatePoint(Matrix4fc rotationMatrixX, Matrix4fc rotationMatrixY, Matrix4fc rotationMatrixZ, Vector4fc point)
	{
		return rotatePoint(Maths.mul(rotationMatrixX, rotationMatrixY).mul(rotationMatrixZ), point);
	}
	
	public static Vector3f rotatePoint(Matrix4fc combinedRotation, Vector3f point)
	{
		return rotatePoint(combinedRotation, new Vector4f(point.x(), point.y(), point.z(), 0));
	}
	
	public static Vector3f rotatePoint(Matrix4fc combinedRotation, Vector4fc point)
	{
		Vector4f vec = new Vector4f(point).mul(combinedRotation);
		return new Vector3f(vec.x, vec.y, vec.z);
	}
	
	/**
	 * Calculates the ending point based off the starting position, rotation values, and the total distance
	 * @param start The starting point
	 * @param v The rotation (z is ignored)
	 * @param dist The total distance travelable
	 * @return The ending point
	 */
	public static Vector3f pointAt(Vector3f start, Vector3f v, float dist)
	{
		return add(toComponents(v.x(), v.y(), dist), start);
	}
	
	/**
	 * Calculates the ending point based off the starting position, rotation values, and the total distance
	 * @param start The starting point
	 * @param rx The x rotation
	 * @param ry The y rotation
	 * @param dist The total distance travelable
	 * @return The ending point
	 */
	public static Vector3f pointAt(Vector3f start, float rx, float ry, float dist)
	{
		return add(toComponents(rx, ry, dist), start);
	}
	
	/**
	 * Calculates the ending point based off the starting position, rotation values, and the total distance
	 * @param rx The x rotation
	 * @param ry The y rotation
	 * @param dist The total distance travelable
	 * @return The ending point
	 */
	public static Vector3f toComponents(float rx, float ry, float velMagnitude)
	{
		Vector3f components = new Vector3f();
		
		final double j = velMagnitude * Math.cos(rx);
		
		components.x = (float) (j * Math.sin(ry));
		components.y = (float) (-velMagnitude * Math.sin(rx));
		components.z = (float) (-j * Math.cos(ry));
		
		return components;
	}
	
	/**
	 * Adds two vectors without modifying either one
	 * @param a The first vector
	 * @param b The second vector
	 * @return A new vector of the two vectors added
	 */
	public static Vector3f add(Vector3fc a, Vector3fc b)
	{
		return new Vector3f(a.x() + b.x(), a.y() + b.y(), a.z() + b.z());
	}
	
	/**
	 * Adds vectors without modifying them
	 * @param vecs The vectors
	 * @return A new vector of two vectors added
	 */
	public static Vector3f add(Vector3f... vecs)
	{
		Vector3f v = Maths.zero();
		
		for(Vector3f c : vecs)
			v.add(c);
		
		return v;
	}
	
	public static Vector3f add(Vector3f v, float x, float y, float z)
	{
		return new Vector3f(v.x() + x, v.y() + y, v.z() + z);
	}
	
	/**
	 * Subtracts two vectors without modifying either one
	 * @param a The first vector
	 * @param b The second vector
	 * @return A new vector of the two vectors subtracted
	 */
	public static Vector3f sub(Vector3f a, Vector3f b)
	{
		return new Vector3f(a.x() - b.x(), a.y() - b.y(), a.z() - b.z());
	}
	
	/**
	 * Subtracts a vector without modifying it
	 * @param a The first vector
	 * @param b The second scalor
	 * @return A new vector of the vector - scalor
	 */
	public static Vector3f sub(Vector3f a, float s)
	{
		return new Vector3f(a.x() - s, a.y() - s, a.z() - s);
	}
	
	/**
	 * Subtracts vectors without modifying them
	 * @param vecs The vectors
	 * @return A new vector of two vectors subtracted
	 */
	public static Vector3f sub(Vector3fc... vecs)
	{
		Vector3f v = Maths.zero();
		
		for(Vector3fc c : vecs)
			v.sub(c);
		
		return v;
	}
	
	/**
	 * Multiplies two vectors without modifying either one
	 * @param a The first vector
	 * @param b The second vector
	 * @return A new vector of the two vectors multiplied
	 */
	public static Vector3f mul(Vector3fc a, Vector3fc b)
	{
		return new Vector3f(a.x() * b.x(), a.y() * b.y(), a.z() * b.z());
	}
	
	/**
	 * Multiplies vectors without modifying them
	 * @param vecs The vectors
	 * @return A new vector of two vectors multiplid
	 */
	public static Vector3f mul(Vector3fc... vecs)
	{
		if(vecs.length == 0)
			return Maths.zero();
		
		Vector3f v = Maths.one();
		
		for(Vector3fc c : vecs)
			v.mul(c);
		
		return v;
	}
	
	/**
	 * Multiplies two vectors without modifying either one
	 * @param x The first vector (<code>new Vector3f(x, x, x)</code>)
	 * @param b The second vector
	 * @return A new vector of the two vectors multiplied
	 */
	public static Vector3f mul(float x, Vector3fc a)
	{
		return mul(a, new Vector3f(x));
	}
	
	/**
	 * Divides two vectors without modifying either one
	 * @param a The first vector
	 * @param b The second vector
	 * @return A new vector of the two vectors divided
	 */
	public static Vector3f div(Vector3fc a, Vector3fc b)
	{
		return new Vector3f(a.x() / b.x(), a.y() / b.y(), a.z() / b.z());
	}
	
	/**
	 * Divides two vectors without modifying either one
	 * @param a The first vector
	 * @param b The second vector
	 * @return A new vector of the two vectors divided
	 */
	public static Vector3f div(Vector3fc a, float d)
	{
		return new Vector3f(a.x() / d, a.y() / d, a.z() / d);
	}
	
	/**
	 * Takes the modulus two vectors without modifying either one
	 * @param a The first vector
	 * @param b The second vector
	 * @return A new vector of the two vectors modulus'ed
	 */
	public static Vector3f mod(Vector3fc a, Vector3fc b)
	{
		return new Vector3f(a.x() % b.x(), a.y() % b.y(), a.z() % b.z());
	}
	
	/**
	 * Takes the modulus two vectors without modifying either one
	 * @param a The first vector
	 * @param b The scalar
	 * @return A new vector of the two vectors modulus'ed
	 */
	public static Vector3f mod(Vector3fc a, float b)
	{
		return new Vector3f(a.x() % b, a.y() % b, a.z() % b);
	}
	
	/**
	 * A Vector3f with all values being 0
	 * @return a Vector3f with all values being 0
	 */
	public static Vector3f zero()
	{
		return new Vector3f(0, 0, 0);
	}
	
	public static Vector3f one()
	{
		return new Vector3f(1, 1, 1);
	}

	public static Vector3f negative()
	{
		return new Vector3f(-1, -1, -1);
	}

	public static float toRads(float degs)
	{
		return Maths.PI * degs / 180f;
	}
	
	public static float toDegs(float rads)
	{
		return rads * 180f / Maths.PI;
	}
	
	public static Vector3f toDegs(Vector3f rads)
	{
		return new Vector3f(toDegs(rads.x()), toDegs(rads.y()), toDegs(rads.z()));
	}

	public static Matrix4f identity()
	{
		return new Matrix4f().identity();
	}

	public static Matrix4f mul(Matrix4fc a, Matrix4fc b) 
	{
		return new Matrix4f().identity().mul(a).mul(b);
	}

	public static Vector3f invert(Vector3f v)
	{
		return new Vector3f(-v.x(), -v.y(), -v.z());
	}

	/**
	 * Same as rotate a by b
	 * @param a Thing to rotate
	 * @param b Thing to be rotated by
	 * @return The rotated vector
	 */
	public static Quaternionf mul(Quaternionfc a, Quaternionfc b)
	{
		return a.mul(b, new Quaternionf());
	}

	/**
	 * Same as un-rotate a by b
	 * @param a Thing to un-rotate
	 * @param b Thing to be un-rotate by
	 * @return The un-rotate vector
	 */
	public static Quaternionf div(Quaternionfc a, Quaternionfc b)
	{
		return a.div(b, new Quaternionf());
	}

	public static Vector3f rotatePoint(Quaternionfc rotation, Vector3f position)
	{
		return new Vector3f(rotation.transform(position, new Vector3f()));
	}

	public static Quaternionfc invert(Quaternionfc q)
	{
		return new Quaternionf().invert();
	}

	public static float clamp(float x, float min, float max)
	{
		return x > max ? max : x < min ? min : x;
	}

	public static Vector3f x(float x)
	{
		return new Vector3f(x, 0, 0);
	}
	
	public static Vector3f y(float y)
	{
		return new Vector3f(0, y, 0);
	}
	
	public static Vector3f z(float z)
	{
		return new Vector3f(0, 0, z);
	}

	public static Quaternionf clone(Quaternionfc rotation)
	{
		return new Quaternionf(rotation.x(), rotation.y(), rotation.z(), rotation.w());
	}
	
	/**
	 * Normalizes a vector ({@link Vector3f#normalize(float)}), but keeps it 0,0,0 if every value is 0
	 * @param vec The vector to normalize
	 * @param max The amount to normalize it to (generally 1)
	 * @return A normalized version of the vector
	 */
	public static Vector3f safeNormalize(Vector3f vec, float max)
	{
		return safeNormalize(vec.x(), vec.y(), vec.z(), max);
	}
	
	public static Vector3f safeNormalize(float x, float y, float z, float max)
	{
		if(x * x + y * y + z * z <= max * max)
			return new Vector3f(x, y, z);
		return new Vector3f(new Vector3f(x, y, z).normalize(max));
	}
	
	public static Vector3f safeNormalizeXZ(Vector3f v, float max)
	{
		Vector3f xzVec = new Vector3f(v.x(), 0, v.z());
		xzVec = safeNormalize(xzVec, max);
		return new Vector3f(xzVec.x(), v.y(), xzVec.z());
	}

	public static Vector3f mul(Vector3fc v, float s)
	{
		return new Vector3f(v.x() * s, v.y() * s, v.z() * s);
	}
	
	public static float sqrt(float x)
	{
		return (float)Math.sqrt(x);
	}
	
	public static float magnitude(Vector3f v)
	{
		return Maths.sqrt(v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
	}
	
	public static Vector3f normalClamp(Vector3f v, float max)
	{
		if(magnitude(v) > max)
			return safeNormalize(v, max);
		else
			return v;
	}

	public static float magnitudeXZ(Vector3f v)
	{
		return Maths.sqrt(v.x() * v.x() + v.z() * v.z());
	}

	public static Vector3f normalClampXZ(Vector3f v, float max)
	{
		if(magnitudeXZ(v) > max)
			return safeNormalizeXZ(v, max);
		else
			return v;
	}
	
	public static Matrix4fc invert(Matrix4fc mat)
	{
		return new Matrix4f(mat).invert();
	}

	public static boolean equals(float a, float b)
	{
		float amb = a - b;
		return amb <= EQUALS_PRECISION && amb >= -EQUALS_PRECISION;
	}
	
	public static boolean equals(Quaternionfc a, Quaternionfc b)
	{
		return equals(a.x(), b.x()) && equals(a.y(), b.y()) && equals(a.z(), b.z()) && equals(a.w(), b.w());
	}

	public static float distSqrd(Vector3f a, Vector3f b)
	{
		float x = a.x() - b.x();
		float y = a.y() - b.y();
		float z = a.z() - b.z();
		
		return x * x + y * y + z * z;
	}
	
	public static float distSqrd(Vector3i a, Vector3f b)
	{
		float x = a.x() - b.x();
		float y = a.y() - b.y();
		float z = a.z() - b.z();
		
		return x * x + y * y + z * z;
	}

	public static float distSqrd(Vector3fc a, Vector3fc b)
	{
		float x = a.x() - b.x();
		float y = a.y() - b.y();
		float z = a.z() - b.z();
		
		return x * x + y * y + z * z;
	}
	
	public static int floor(float x)
	{
		return (int)Math.floor(x);
	}
	
	public static int min(int a, int b)
	{
		return a < b ? a : b;
	}

	public static float min(float a, float b)
	{
		return a < b ? a : b;
	}
	
	public static int max(int a, int b)
	{
		return a > b ? a : b;
	}
	
	public static float max(float a, float b)
	{
		return a > b ? a : b;
	}

	public static int round(float x)
	{
		return Math.round(x);
	}

	public static float signum0(float z)
	{
		return z == 0 ? 1 : Math.signum(z);
	}

	public static Vector3f vec3(Vector4fc temp1)
	{
		return new Vector3f(temp1.x(), temp1.y(), temp1.z());
	}

}
