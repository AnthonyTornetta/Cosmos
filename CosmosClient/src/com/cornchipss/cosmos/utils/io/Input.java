package com.cornchipss.cosmos.utils.io;

import org.lwjgl.glfw.GLFW;

import com.cornchipss.cosmos.rendering.Window;

public class Input
{
	private static Window window;
	
	private static KeyListener keyListener;
	private static MouseListener mouseListener;
	
	private Input() { throw new IllegalStateException("Cannot instantiate the Input class!"); }
	
	public static void update()
	{
		keyListener.update();
		mouseListener.update();
	}
	
	public static void setWindow(Window window)
	{
		if(Input.window != null)
			throw new IllegalStateException("Input handler already initialized to a window!");
		
		Input.window = window;
		
		keyListener = new KeyListener();
		mouseListener = new MouseListener(window.getId());
		
		GLFW.glfwSetMouseButtonCallback(window.getId(), mouseListener);
		GLFW.glfwSetKeyCallback(window.getId(), keyListener);
	}

	public static boolean isKeyDown(int key)
	{
		return keyListener.isKeyDown(key);
	}
	
	public static boolean isKeyJustDown(int key)
	{
		return keyListener.isKeyJustDown(key);
	}
	
	public static boolean isMouseBtnDown(int mouseBtn)
	{
		return mouseListener.isBtnDown(mouseBtn);
	}
	
	public static boolean isMouseBtnJustDown(int mouseBtn)
	{
		return mouseListener.isBtnJustDown(mouseBtn);
	}
	
	public static void hideCursor(boolean hide)
	{
		if(hide)
		{
			GLFW.glfwSetInputMode(window.getId(), GLFW.GLFW_CURSOR, GLFW.GLFW_CURSOR_DISABLED);
		}
		else
		{
			GLFW.glfwSetInputMode(window.getId(), GLFW.GLFW_CURSOR, GLFW.GLFW_CURSOR_NORMAL);
		}
	}
	
	public static float getMouseX() { return mouseListener.getMouse().x; }
	public static float getMouseY() { return mouseListener.getMouse().y; }
	public static float getMouseDeltaX() { return mouseListener.getMouse().deltaX; }
	public static float getMouseDeltaY() { return mouseListener.getMouse().deltaY; }

	public static void toggleCursor()
	{
		hideCursor(GLFW.glfwGetInputMode(window.getId(), GLFW.GLFW_CURSOR) == GLFW.GLFW_CURSOR_NORMAL);
	}
}
