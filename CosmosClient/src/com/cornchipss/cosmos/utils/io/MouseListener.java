package com.cornchipss.cosmos.utils.io;

import java.util.LinkedList;
import java.util.List;

import org.lwjgl.glfw.GLFW;
import org.lwjgl.glfw.GLFWMouseButtonCallbackI;

public class MouseListener implements GLFWMouseButtonCallbackI
{
	private boolean[] mouseButtonsDown = new boolean[GLFW.GLFW_MOUSE_BUTTON_LAST + 1];
	private boolean[] mouseBtnsJustDown = new boolean[GLFW.GLFW_MOUSE_BUTTON_LAST + 1];
	
	private List<Integer> mouseButtonsJustPressed = new LinkedList<>();
	
	private final Mouse mouse = new Mouse();
	
	public static class Mouse // No getters/setters because it's a struct
	{
		public float x, y, deltaX, deltaY;
	}
	
	private long window;
	
	public MouseListener(long window)
	{
		this.window = window;
	}
	
	@Override
	public void invoke(long window, int mouseBtn, int action, int arg3)
	{
		if(mouseBtn == GLFW.GLFW_KEY_UNKNOWN)
			return;
		
		if(action == GLFW.GLFW_PRESS)
		{
			mouseButtonsDown[mouseBtn] = true;
			mouseBtnsJustDown[mouseBtn] = true;
			
			mouseButtonsJustPressed.add(mouseBtn);
		}
		else
			mouseButtonsDown[mouseBtn] = false;
	}

	public boolean isBtnDown(int mouseBtn)
	{
		return mouseButtonsDown[mouseBtn];
	}
	
	public boolean isBtnJustDown(int mouseBtn)
	{
		return mouseBtnsJustDown[mouseBtn];
	}

	public void update()
	{
		for(int i : mouseButtonsJustPressed)
			mouseBtnsJustDown[i] = false;
		mouseButtonsJustPressed.clear();
		
		double[] mouseX = new double[1];
		double[] mouseY = new double[1];
		GLFW.glfwGetCursorPos(window, mouseX, mouseY);
		mouse.deltaX = (float) (mouse.x - mouseX[0]);
		mouse.deltaY = (float) (mouse.y - mouseY[0]);
		
		mouse.x = (float)mouseX[0];
		mouse.y = (float)mouseY[0];
	}

	public Mouse getMouse()
	{
		return mouse;
	}
}
