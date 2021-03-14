package com.cornchipss.cosmos.utils.io;

import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;

public interface IWritable
{
	public void write(DataOutputStream writer) throws IOException;
	public void read(DataInputStream reader) throws IOException;
}
