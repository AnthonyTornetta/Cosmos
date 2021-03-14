package com.cornchipss.cosmos.utils;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.FloatBuffer;
import java.util.List;

import org.joml.Matrix4fc;
import org.joml.Quaternionfc;
import org.joml.Vector3dc;
import org.joml.Vector3f;
import org.joml.Vector3fc;
import org.joml.Vector3ic;
import org.joml.Vector4fc;

public class Utils
{
	/**
	 * Buffer purely for holding mitricies. Pre-allocated to save some runtime processes
	 */
	private static FloatBuffer matrixBuffer = ByteBuffer.allocate(4 * 16).order(ByteOrder.nativeOrder()).asFloatBuffer();
	
	private static final Vector3fc X_VECTOR = new Vector3f(1, 0, 0), Y_VECTOR = new Vector3f(0, 1, 0), Z_VECTOR = new Vector3f(0, 0, 1);
	
	/**
	 * Prints an object w/ the class & line number next to it and a newline
	 * @param obj The object to print
	 */
	public static void println(Object obj)
	{
		printraw(toString(obj) + "\n");
	}
	
	/**
	 * Prints objects w/ the class & line number next to it and a newline
	 * @param obj The objects to print
	 */
	public static void println(Object... obj)
	{
		printraw(toString(obj) + "\n");
	}
	
	/**
	 * Prints an object w/ the class & line number next to it
	 * @param obj The object to print
	 */
	public static void print(Object obj)
	{
		printraw(toString(obj));
	}
	
	private static void printraw(String s)
	{		
		StackTraceElement trace = Thread.currentThread().getStackTrace()[3];
		
		String clazz = trace.getClassName();

		System.out.print(clazz.substring(clazz.lastIndexOf(".") + 1) + " (" + trace.getLineNumber() + ")> " + s);
	}
	
	public static void printCaller()
	{
		printraw(caller() + "\n");
	}
	
	public static String caller()
	{
		StackTraceElement trace = Thread.currentThread().getStackTrace()[4];
		String clazz = trace.getClassName();
		return clazz.substring(clazz.lastIndexOf(".") + 1) + " (" + trace.getLineNumber() + ")";
	}
	
	public static void printStackTrace()
	{
		Thread.dumpStack();
	}
	
	/**
	 * Nicely puts an array into a String
	 * @param obj The array
	 * @return The Stringified array
	 */
	public static String arrayToString(Object[] obj)
	{
		if(obj == null)
			return "null";
		
		StringBuilder builder = new StringBuilder("[");
		
		for(int i = 0; i < obj.length; i++)
		{
			if(i != 0)
				builder.append(", ");
			
			builder.append(toString(obj[i]));
		}
		
		builder.append("]");
		
		return builder.toString();
	}
	
	/**
	 * .toString()++
	 * @param obj The object to toString()
	 * @return A nice String
	 */
	public static String toString(Object obj)
	{
		if(obj == null)
			return "null";
		
		if(obj.getClass().isArray())
		{
			try
			{
				Object[] arr = (Object[])obj;
				return arrayToString(arr);
			}
			catch(ClassCastException ex)
			{
				return obj.toString();
			}
		}
		
		if(obj instanceof Quaternionfc)
		{
			Quaternionfc q = (Quaternionfc)obj;
			return "[" + q.x() + ", " + q.y() + ", " + q.z() + ", " + q.w() + "]";//toString(Maths.toDegs(eulers));
		}
		
		if(obj instanceof Vector3fc)
		{
			Vector3fc t = (Vector3fc)obj;
			return "[" + t.x() + ", " + t.y() + ", " + t.z() + "]";
		}
		else if(obj instanceof Vector3ic)
		{
			Vector3ic t = (Vector3ic)obj;
			return "[" + t.x() + ", " + t.y() + ", " + t.z() + "]";
		}
		else if(obj instanceof Vector3dc)
		{
			Vector3dc t = (Vector3dc)obj;
			return "[" + t.x() + ", " + t.y() + ", " + t.z() + "]";
		}
		else if(obj instanceof Vector4fc)
		{
			Vector4fc t = (Vector4fc)obj;
			return "[" + t.x() + ", " + t.y() + ", " + t.z() + ", " + t.w() + "]";
		}
		
		return obj.toString();
	}
	
	/**
	 * Converts a List of Floats into a primitive array of floats
	 * @param list The list
	 * @return a primitive array of floats
	 */
	public static float[] toArray(List<Float> list)
	{
		float[] arr = new float[list.size()];
		
		int i = 0;
		for(float f : list) // Avoids multiple O(n) calculations w/ linked lists
		{
			arr[i++] = f;
		}
		
		return arr;
	}

	/**
	 * Converts a List of Integers into a primitive array of integers
	 * @param list The list
	 * @return a primitive array of integers
	 */
	public static int[] toArrayInt(List<Integer> list)
	{
		int[] arr = new int[list.size()];
		
		int i = 0;
		for(int num : list) // Avoids multiple O(n) calculations w/ linked lists
		{
			arr[i++] = num;
		}
		
		return arr;
	}
	
	/**
	 * <p>Not Thread Safe - the FloatBuffer used is pre-allocated in memory and is a static variable to this class</p>
	 * <p>Puts a matrix into a FloatBuffer.</p>
	 * @param mat The Matrix to use
	 * @return The matrix in a FloatBuffer
	 */
	public static FloatBuffer toFloatBuffer(Matrix4fc mat)
	{
		mat.get(matrixBuffer);
		return matrixBuffer;
	}
	
	/**
	 * Converts 3D array coordinates to 1D array coordinates, useful for parallel arrays but with a 3D and 1D
	 * @param x The x in the 3D array
	 * @param y The y in the 3D array
	 * @param z The z in the 3D array
	 * @param width The width of the array
	 * @param height The height of the array
	 * @return The index that would be in a 1D array
	 */
	public static int array3Dto1D(int x, int y, int z, int width, int height)
	{
		return x + y * width + z * width * height;
	}
	
	/**
	 * Clamps a number between two other numbers
	 * @param x The number to clamp between two others
	 * @param m The minimum number
	 * @param M The maximum number
	 * @return The number within <code>m</code> and <code>M</code> inclusive
	 */
	public static float clamp(float x, float m, float M)
	{
		return x > M ? M : x < m ? m : x;
	}

	/**
	 * Checks if an array contains an object o
	 * @param arr The array
	 * @param o The object
	 * @return if an array contains an object o
	 */
	public static boolean contains(Object[] arr, Object o)
	{
		for(Object t : arr)
			if(t == null && o == null || t != null && t.equals(o))
				return true;
		return false;
	}
	
	/**
	 * Checks if an array contains an object o
	 * @param list The list
	 * @param o The object
	 * @return if an array contains an object o
	 */
	public static<T> boolean contains(List<T> list, T o)
	{
		for(T t : list)
			if(t == null && o == null || t != null && t.equals(o))
				return true;
		return false;
	}
	
	/**
	 * Checks if two objects are equal & is null safe
	 * @param a Object A
	 * @param b Object B
	 * @return Checks if two objects are equal
	 */
	public static boolean equals(Object a, Object b)
	{
		if(a == null && b != null)
			return false;
		if(a != null && b == null)
			return false;
		if(a == null && b == null)
			return true;
		   
		return a.equals(b);
	}

	public static Vector3fc x() { return X_VECTOR; }
	public static Vector3fc y() { return Y_VECTOR; }
	public static Vector3fc z() { return Z_VECTOR; }
	
	public static String toEasyString(Vector3fc v)
	{
		return "[" + toEasyString(v.x()) + ", " + toEasyString(v.y()) + ", " + toEasyString(v.z()) + "]";
	}
	
	public static String toEasyString(float f)
	{
		return (Math.round(f * 10) / 10.0f) + "";
	}
}
