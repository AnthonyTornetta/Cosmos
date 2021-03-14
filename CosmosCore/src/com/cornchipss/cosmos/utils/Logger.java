package com.cornchipss.cosmos.utils;

public class Logger
{
	public static final Logger LOGGER = new Logger(LogLevel.INFO);
	
	public static enum LogLevel
	{
		DEBUG(0),
		INFO(1),
		WARNING(2),
		ERROR(3),
		NONE(4);
		
		int val;
		
		LogLevel(int v)
		{
			val = v;
		}
	}
	
	private LogLevel level;
	
	public Logger(LogLevel level)
	{
		this.level = level;
	}
	
	public void setLevel(LogLevel lvl)
	{
		this.level = lvl;
	}
	
	private String raw(String msg, String level)
	{
		return "[" + level + "] [" + traceInfo() + "] " + msg;
	}
	
	private String traceInfo()
	{
		StackTraceElement trace = Thread.currentThread().getStackTrace()[4];
		
		String clazz = trace.getClassName();
		
		return clazz.substring(clazz.lastIndexOf(".") + 1) + ":" + trace.getLineNumber() + "";
	}
	
	public void debug(Object msg)
	{
		if(level.val <= LogLevel.DEBUG.val)
			System.out.println(raw(msg != null ? Utils.toString(msg) : "null", "Debug"));
	}
	
	public void info(Object msg)
	{
		if(level.val <= LogLevel.INFO.val)
			System.out.println(raw(msg != null ? Utils.toString(msg) : "null", "Info"));
	}
	
	public void warning(Object msg)
	{
		if(level.val <= LogLevel.WARNING.val)
			System.out.println(raw(msg != null ? Utils.toString(msg) : "null", "Warning"));
	}
	
	public void error(Object msg)
	{
		if(level.val <= LogLevel.ERROR.val)
			System.out.println(raw(msg != null ? Utils.toString(msg) : "null", "Error"));
	}
}
