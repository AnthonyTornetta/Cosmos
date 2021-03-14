package com.cornchipss.cosmos.rendering;

import java.awt.image.BufferedImage;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.ByteBuffer;

import org.lwjgl.BufferUtils;
import org.lwjgl.opengl.GL11;
import org.newdawn.slick.opengl.PNGDecoder;

public class Texture
{
	private int id;
	
	public Texture(int id)
	{
		this.id = id;
	}
	
	public static Texture loadTexture(BufferedImage image)
	{
		ByteBuffer buffer = ByteBuffer.allocateDirect(image.getWidth() * image.getHeight() * 4);
		
	    for(int h = 0; h < image.getHeight(); h++)
	    {
	        for(int w = 0; w < image.getWidth(); w++)
	        {
	            int pixel = image.getRGB(w, h);

	            buffer.put((byte) ((pixel >> 16) & 0xFF));
	            buffer.put((byte) ((pixel >> 8) & 0xFF));
	            buffer.put((byte) (pixel & 0xFF));
	            buffer.put((byte) ((pixel >> 24) & 0xFF));
	        }
	    }
	    
	    buffer.flip();
	    
	    return loadTexture(buffer, image.getWidth(), image.getHeight());
	}
	
	public static Texture loadTexture(String texture)
	{
		try
		{
			PNGDecoder decoder = new PNGDecoder(new FileInputStream(texture + ".png"));
			ByteBuffer buffer = BufferUtils.createByteBuffer(decoder.getWidth() * decoder.getHeight() * 4); //4 -> rgba
			decoder.decode(buffer, decoder.getWidth() * 4, PNGDecoder.RGBA);
			buffer.rewind();
			
			return loadTexture(buffer, decoder.getWidth(), decoder.getHeight());
		}
		catch (IOException e)
		{
			e.printStackTrace();
			return null;
		}
	}
	
	private static Texture loadTexture(ByteBuffer buffer, int w, int h)
	{
		int id = GL11.glGenTextures();
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, id);
		
		GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MIN_FILTER, GL11.GL_NEAREST);
		GL11.glTexParameteri(GL11.GL_TEXTURE_2D, GL11.GL_TEXTURE_MAG_FILTER, GL11.GL_NEAREST);
		
		GL11.glTexImage2D(GL11.GL_TEXTURE_2D, 0, GL11.GL_RGBA, w, h, 0, GL11.GL_RGBA, GL11.GL_UNSIGNED_BYTE, buffer);
		
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, 0);
		
		return new Texture(id);
	}
	
	public void bind()
	{
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, getId());
	}
	
	public static void unbind()
	{
		GL11.glBindTexture(GL11.GL_TEXTURE_2D, 0);
	}
	
	public int getId() { return id; }
}
