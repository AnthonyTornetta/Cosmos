package com.cornchipss.cosmos.utils;

import java.util.HashMap;
import java.util.Map;

public class DebugMonitor
{
	private static Map<String, Object> debugValues = new HashMap<>();
	
	private DebugMonitor() {}
	
	public static Object get(String key)
	{
		return debugValues.get(key);
	}
	
	public static void set(String key, Object value)
	{
		debugValues.put(key, value);
	}
	
	public static boolean has(String key)
	{
		return debugValues.containsKey(key);
	}
}
