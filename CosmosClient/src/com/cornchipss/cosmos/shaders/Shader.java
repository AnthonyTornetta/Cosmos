package com.cornchipss.cosmos.shaders;

import java.io.BufferedReader;
import java.io.FileReader;

import org.joml.Matrix4fc;
import org.lwjgl.opengl.GL20;
import org.lwjgl.opengl.GL30;

import com.cornchipss.cosmos.utils.Logger;

public class Shader
{
	private String shaderLocation;
	
	private int programID;
	
	private float[] floatBuffer;
	
	public Shader(String loc)
	{
		shaderLocation = loc;
	}
	
	public int programID()
	{
		return programID;
	}
	
	public int uniformLocation(String name)
	{
		return GL20.glGetUniformLocation(programID, name);
	}
	
	public void use()
	{
		GL30.glUseProgram(programID);
	}
	
	public void stop()
	{
		GL30.glUseProgram(0);
	}
	
	public void setUniformF(int location, float value)
	{
		GL30.glUniform1f(location, value);
	}
	
	public void setUniformI(int location, int value)
	{
		GL30.glUniform1i(location, value);
	}
	
	public void setUniformVector(int location, float x, float y, float z)
	{
		GL30.glUniform3f(location, x, y, z);
	}
	
	public void setUniformMatrix(int location, Matrix4fc mat)
	{
		GL30.glUniformMatrix4fv(location, false, mat.get(floatBuffer));
	}
	
	public void init()
	{
		floatBuffer = new float[16];
		
		int vert = load(shaderLocation + ".vert", GL30.GL_VERTEX_SHADER);
		int frag = load(shaderLocation + ".frag", GL30.GL_FRAGMENT_SHADER);
		
		programID = GL30.glCreateProgram();
		GL30.glAttachShader(programID, vert);
		GL30.glAttachShader(programID, frag);
		GL30.glLinkProgram(programID);
		
		Logger.LOGGER.info("Validating Shader...");
		GL20.glValidateProgram(programID);
		
		Logger.LOGGER.info("Shader Loader > " + GL30.glGetProgramInfoLog(programID));
		
		if(GL30.glGetProgrami(programID, GL30.GL_LINK_STATUS) == 0)
		{
			String log = GL30.glGetProgramInfoLog(programID);
			Logger.LOGGER.error("Shader Program Linking Error!!!");
			Logger.LOGGER.error(log);
			System.exit(-1);
		}
		
		// Once they are linked to the program, we do not need them anymore.
		GL30.glDeleteShader(vert);
		GL30.glDeleteShader(frag);
	}
	
	private int load(String shaderLocation, int type)
	{
		StringBuilder shaderCode = new StringBuilder();
		BufferedReader br = null;
		try
		{
			br = new BufferedReader(new FileReader(shaderLocation));
			
			for(String line = br.readLine(); line != null; line = br.readLine())
			{
				shaderCode.append(line + System.lineSeparator());
			}
			
			br.close();
		}
		catch(Exception ex)
		{
			throw new RuntimeException(ex);
		}
		
		int shader = GL30.glCreateShader(type);
		GL30.glShaderSource(shader, shaderCode.toString());
		GL30.glCompileShader(shader);
		
		int success = GL30.glGetShaderi(shader, GL30.GL_COMPILE_STATUS);
		if(success == 0)
		{
			String log = GL30.glGetShaderInfoLog(shader);
			Logger.LOGGER.error("Vertex Shader Compilation Error!!!");
			Logger.LOGGER.error(log);
			System.exit(-1);
		}
		
		return shader;
	}
}
