package com.cornchipss.cosmos.gui.text;

import javax.annotation.Nonnull;

import com.cornchipss.cosmos.rendering.Mesh;

public class TextRenderer
{
	private final OpenGLFont font;
	
	public TextRenderer(@Nonnull OpenGLFont f)
	{
		font = f;
	}
	
	public static Mesh createMesh(String text, OpenGLFont font)
	{
		return createMesh(text, font, 0, 0);
	}
	
	public static Mesh createMesh(String text, OpenGLFont font, float x, float y)
	{
		float xOff = 0;
		float yOff = 0;
		
		int[] indicies = new int[text.length() * 6];
		float[] uvs = new float[text.length() * 8];
		float[] vertices = new float[text.length() * 12];
		
		for(int i = 0; i < text.length(); i++)
		{
			char c = text.charAt(i);
			
			if(c == '\n')
			{
				yOff += font.height();
				xOff = 0;
			}
			
			float charLoc = font.uBegin(c);
			float nextCharLoc = font.uEnd(c);
			float width = font.charWidth(c);
			
			indicies[i * 6 + 0] = i * 4;
			indicies[i * 6 + 1] = i * 4 + 1;
			indicies[i * 6 + 2] = i * 4 + 3;
			
			indicies[i * 6 + 3] = i * 4 + 1;
			indicies[i * 6 + 4] = i * 4 + 2;
			indicies[i * 6 + 5] = i * 4 + 3;
			
			uvs[i * 8 + 0] = nextCharLoc;
			uvs[i * 8 + 1] = 0;
			
			uvs[i * 8 + 2] = nextCharLoc;
			uvs[i * 8 + 3] = 1;
			
			uvs[i * 8 + 4] = charLoc;
			uvs[i * 8 + 5] = 1;
			
			uvs[i * 8 + 6] = charLoc;
			uvs[i * 8 + 7] = 0;
			
			vertices[i * 12 + 0] = xOff + x + width;
			vertices[i * 12 + 1] = yOff + y + font.height();
			vertices[i * 12 + 2] = 0;
			
			vertices[i * 12 + 3] = xOff + x + width;
			vertices[i * 12 + 4] = yOff + y;
			vertices[i * 12 + 5] = 0;
			
			vertices[i * 12 + 6] = xOff + x;
			vertices[i * 12 + 7] = yOff + y;
			vertices[i * 12 + 8] = 0;
			
			vertices[i * 12 + 9] = xOff + x;
			vertices[i * 12 + 10]= yOff + y + font.height();
			vertices[i * 12 + 11]= 0;
			
			xOff += width;
		}
		
		return Mesh.createMesh(vertices, indicies, uvs);
	}
	
	/**
	 * Renders text, but in an inefficient way<br>
	 * See {@link GUIText} instead
	 * @param text Text to render
	 * @param x X position
	 * @param y Y position
	 */
	public void renderText(String text, float x, float y)
	{
		Mesh m = createMesh(text, font, x, y);
		
		font.bind();
		
		m.prepare();
		m.draw();
		m.finish();
		
		OpenGLFont.unbind();
		
		m.delete();
	}
}
