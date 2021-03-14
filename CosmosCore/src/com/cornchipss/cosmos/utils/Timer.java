package com.cornchipss.cosmos.utils;

public class Timer
{
	private long lastTime, offset;
	
	public Timer()
	{
		reset();
	}
	
	public long getDelta()
	{
		return System.nanoTime() - lastTime + offset;
	}
	
	public long getDeltaMillis()
	{
		return getDelta() / 1_000_000;
	}
	
	public void reset()
	{
		lastTime = System.nanoTime();
		offset = 0;
	}
	
	public void subtractTimeNano(long time)
	{
		offset -= time;
	}
	
	public void subtractTimeMilli(long time)
	{
		subtractTimeNano(time * 1_000_000);;
	}

	public static void sleep(long millis)
	{
		if(millis > 0)
		{
			try
			{
				Thread.sleep(millis);
			}
			catch (InterruptedException e)
			{
				e.printStackTrace();
			}
		}
	}
}
