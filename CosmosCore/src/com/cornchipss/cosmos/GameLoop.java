package com.cornchipss.cosmos;

import com.cornchipss.cosmos.utils.DebugMonitor;

public abstract class GameLoop implements Runnable
{	
	private boolean running = true;
	
	public abstract void update(float delta);
	
	@Override
	public void run()
	{
		long t = System.currentTimeMillis();
		
		final int UPS_TARGET = 70;
		
		final int MILLIS_WAIT = 1000 / UPS_TARGET;
		
		long lastSecond = t;
		
		int ups = 0;
		float variance = 0;
		
		DebugMonitor.set("ups", 0);
		DebugMonitor.set("ups-variance", 0.0f);
		
		while(running)
		{
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
			
			update(delta);
		}
	}
	
	public boolean running() { return running; }
	public void running(boolean b) { running = b; }
}
