package com.cornchipss.cosmos.rendering;

import java.nio.FloatBuffer;
import java.nio.IntBuffer;
import java.util.LinkedList;
import java.util.List;

import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL15;
import org.lwjgl.opengl.GL20;
import org.lwjgl.opengl.GL30;

public class Mesh
{
	public static final int VERTEX_INDEX = 0;
	public static final int COLOR_INDEX  = 1;
	public static final int UV_INDEX     = 2;
	public static final int LIGHT_INDEX  = 3;
	
	final private int vao;
	final private int verticies;
	
	private List<Integer> vbos; // for deleting them when done with this mesh.
	
	private Mesh(int verticies)
	{
		vao = GL30.glGenVertexArrays(); 
		this.verticies = verticies;
		vbos = new LinkedList<>();
	}
	
	public void storeData(int index, int dimensions, float[] data)
	{
		int vbo = GL15.glGenBuffers();
		vbos.add(vbo);
		GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, vbo);
		
		FloatBuffer dataBuffer = BufferUtils.createFloatBuffer(data.length);
		dataBuffer.put(data);
		dataBuffer.flip();

		GL15.glBufferData(GL15.GL_ARRAY_BUFFER, dataBuffer, GL15.GL_STATIC_DRAW);
		GL30.glVertexAttribPointer(index, dimensions, GL11.GL_FLOAT, false, 0, 0);
		
		GL15.glBindBuffer(GL15.GL_ARRAY_BUFFER, 0);
	}
	
	public void storeIndicies(int[] data)
	{
		int vbo = GL15.glGenBuffers();
		vbos.add(vbo);
		GL15.glBindBuffer(GL15.GL_ELEMENT_ARRAY_BUFFER, vbo);
		
		IntBuffer buf = BufferUtils.createIntBuffer(data.length);
		buf.put(data);
		buf.flip();
		
		GL15.glBufferData(GL15.GL_ELEMENT_ARRAY_BUFFER, buf, GL15.GL_STATIC_DRAW);
	}
	
	public static Mesh createMesh(float[] verticies, int[] indicies, float[] uvs)
	{
		Mesh m = new Mesh(indicies.length);
		GL30.glBindVertexArray(m.vao());
		
		m.storeData(VERTEX_INDEX, 3, verticies);
		
		m.storeIndicies(indicies);
		
		m.storeData(UV_INDEX, 2, uvs);
		
		// hey idiot. are you adding something and it's not working? make sure you enable all the required GL buffers when you draw it.

		GL30.glBindVertexArray(0);
		return m;
	}
	
	public static Mesh createMesh(float[] verticies, int[] indicies, float[] uvs, float[] lightsArr, boolean unbind)
	{
		Mesh m = new Mesh(indicies.length);
		GL30.glBindVertexArray(m.vao());
		
		m.storeData(VERTEX_INDEX, 3, verticies);
		
		m.storeIndicies(indicies);
		
		m.storeData(UV_INDEX, 2, uvs);
		
		m.storeData(LIGHT_INDEX, 3, lightsArr);
		
		// hey idiot. are you adding something and it's not working? make sure you enable all the required GL buffers when you draw it.
		
		if(unbind)
			GL30.glBindVertexArray(0);
		
		return m;
	}
	
	public static Mesh createMesh(float[] verticies, int[] indicies, float[] uvs, float[] lightsArr)
	{
		return createMesh(verticies, indicies, uvs, lightsArr, true);
	}

	public int vao()
	{
		return vao;
	}
	
	public int verticies()
	{
		return verticies;
	}

	public void prepare()
	{
		GL30.glBindVertexArray(vao());
		
		GL20.glEnableVertexAttribArray(0);
		GL20.glEnableVertexAttribArray(1);
		GL20.glEnableVertexAttribArray(2);
		GL20.glEnableVertexAttribArray(3);
		GL20.glEnableVertexAttribArray(4);
	}

	public void finish()
	{
		GL20.glDisableVertexAttribArray(4);
		GL20.glDisableVertexAttribArray(3);
		GL20.glDisableVertexAttribArray(2);
		GL20.glDisableVertexAttribArray(1);
		GL20.glDisableVertexAttribArray(0);
		
		GL30.glBindVertexArray(0);
	}
	
	public void draw()
	{
		GL11.glDrawElements(GL20.GL_TRIANGLES, verticies(), GL11.GL_UNSIGNED_INT, 0);
	}
	
	public void delete()
	{
		GL30.glBindVertexArray(vao());
		
		for(int vbo : vbos)
			GL30.glDeleteBuffers(vbo);
		
		GL30.glDeleteVertexArrays(vao());
		
		GL30.glBindVertexArray(0);
	}

	public void unbind()
	{
		GL30.glBindVertexArray(0);
	}
}
