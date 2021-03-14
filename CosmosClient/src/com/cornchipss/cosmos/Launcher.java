package com.cornchipss.cosmos;

import org.lwjgl.glfw.GLFW;

import com.cornchipss.cosmos.game.ClientGame;
import com.cornchipss.cosmos.registry.Biospheres;
import com.cornchipss.cosmos.registry.DesertBiosphere;
import com.cornchipss.cosmos.registry.GrassBiosphere;
import com.cornchipss.cosmos.registry.Initializer;
import com.cornchipss.cosmos.rendering.Window;
import com.cornchipss.cosmos.utils.DebugMonitor;
import com.cornchipss.cosmos.utils.Logger;
import com.cornchipss.cosmos.utils.io.Input;

public class Launcher
{
	private Window window;
	
	public static void main(String[] args)
	{
		new Launcher().run();
	}
	
	private void run()
	{
		Logger.LOGGER.setLevel(Logger.LogLevel.DEBUG);
		
		window = new Window(1024, 720, "Cosmos");
		
		/*
		 * 
	{
		Logger.LOGGER.info("Initializing...");
		
		Blocks.init();
		
		Biospheres.registerBiosphere(GrassBiosphere.class, "cosmos:grass");
		Biospheres.registerBiosphere(DesertBiosphere.class, "cosmos:desert");

		Materials.initMaterials();
		
		Logger.LOGGER.info("Initialization Complete");
	}
		 */
		Initializer loader = new Initializer();
		loader.init();
		
		ClientGame game = new ClientGame(window);
		
		Input.setWindow(window);
		
		long t = System.currentTimeMillis();
		
		final int UPS_TARGET = 70;
		
		final int MILLIS_WAIT = 1000 / UPS_TARGET;
		
		long lastSecond = t;
		
		int ups = 0;
		float variance = 0;
		
		Input.hideCursor(true);
		
		Input.update();
		
		boolean running = true;
		
		DebugMonitor.set("ups", 0);
		DebugMonitor.set("ups-variance", 0.0f);
		
		while(!window.shouldClose() && running)
		{
			if(window.wasWindowResized())
				game.onResize(window.getWidth(), window.getHeight());
			
			float delta = System.currentTimeMillis() - t; 
			
			if(delta < MILLIS_WAIT)
			{
				try
				{
					Thread.sleep(MILLIS_WAIT - (int)delta);
					
					delta = (System.currentTimeMillis() - t);
				}
				catch (InterruptedException e)
				{
					e.printStackTrace();
				}
			}
			
			delta /= 1000.0f;
			
			if(delta > variance)
				variance = delta;
			
			t = System.currentTimeMillis();
			
			if(lastSecond / 1000 != t / 1000)
			{
				DebugMonitor.set("ups", ups);
				DebugMonitor.set("ups-variance", variance);
				
				lastSecond = t;
				ups = 0;
				variance = 0;
			}
			ups++;
			
			if(Input.isKeyJustDown(GLFW.GLFW_KEY_F1))
				Input.toggleCursor();
			
			if(Input.isKeyDown(GLFW.GLFW_KEY_ESCAPE))
				running = false;
			
			game.update(delta);
			
			Input.update();
			
			window.clear(0, 0, 0, 1);
			
			game.render(delta);
			
			window.update();
		}
		
		window.destroy();
		
		Logger.LOGGER.info("Successfully closed.");
	}
}
