package com.cornchipss.cosmos.server;

import com.cornchipss.cosmos.GameLoop;
import com.cornchipss.cosmos.game.Game;

public class ServerLoop extends GameLoop
{
	private Game game;
	
	public ServerLoop(Game game)
	{
		this.game = game;
	}
	
	@Override
	public void update(float delta)
	{
		game.update(delta);
	}
}
