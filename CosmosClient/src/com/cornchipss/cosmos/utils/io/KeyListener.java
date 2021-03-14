package com.cornchipss.cosmos.utils.io;

import java.util.LinkedList;
import java.util.List;

import org.lwjgl.glfw.GLFW;
import org.lwjgl.glfw.GLFWKeyCallbackI;

public class KeyListener implements GLFWKeyCallbackI
{
	private boolean[] keysDown = new boolean[GLFW.GLFW_KEY_LAST + 1];
	private boolean[] keysJustDown = new boolean[GLFW.GLFW_KEY_LAST + 1];
	
	private List<Integer> keysPressed = new LinkedList<>();
	
	@Override
	public void invoke(long window, int key, int arg2, int action, int arg4)
	{
		if(key == GLFW.GLFW_KEY_UNKNOWN)
			return;
		
		if(action == GLFW.GLFW_PRESS)
		{
			keysDown[key] = true;
			keysJustDown[key] = true;
			keysPressed.add(key);
		}
		else if(action == GLFW.GLFW_RELEASE)
			keysDown[key] = false;
		else if(action == GLFW.GLFW_REPEAT)
		{
			
		}
	}

	public void update()
	{
		for(int i : keysPressed)
			keysJustDown[i] = false;
		keysPressed.clear();
	}
	
	public boolean isKeyDown(int key)
	{
		return keysDown[key];
	}

	public boolean isKeyJustDown(int key)
	{
		return keysJustDown[key];
	}
}
