package com.cornchipss.cosmos.server;

import java.util.Scanner;

import com.cornchipss.cosmos.registry.Initializer;
import com.cornchipss.cosmos.server.game.ServerGame;
import com.cornchipss.cosmos.server.registry.ServerInitializer;
import com.cornchipss.cosmos.utils.Logger;

public class ServerLauncher
{
	public static void main(String[] args)
	{
		new ServerLauncher().run();
	}
	
	private void run()
	{
		Logger.LOGGER.setLevel(Logger.LogLevel.DEBUG);
		
		Initializer loader = new ServerInitializer();
		loader.init();
		
		ServerGame game = new ServerGame();
		
		ServerLoop gl = new ServerLoop(game);
		
		Thread gameThread = new Thread(gl);
		gameThread.start();
		Logger.LOGGER.info("Game thread started...");
		
		Scanner scan = new Scanner(System.in);
		scan.nextLine();
		scan.close();
		gl.running(false);
		
		try
		{
			gameThread.join();
			Logger.LOGGER.info("Game thread ended...");
		}
		catch (InterruptedException e)
		{
			e.printStackTrace();
		}
		
		Logger.LOGGER.info("Successfully closed.");
	}
}
