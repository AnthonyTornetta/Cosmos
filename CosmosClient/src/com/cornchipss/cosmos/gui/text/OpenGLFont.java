package com.cornchipss.cosmos.gui.text;

import java.awt.Color;
import java.awt.Font;
import java.awt.FontMetrics;
import java.awt.Graphics2D;
import java.awt.image.BufferedImage;

import javax.annotation.Nonnull;

import com.cornchipss.cosmos.rendering.Texture;

public class OpenGLFont
{
	/**
	 * The minimum/maximum ASCII codes of the characters supported - {@link OpenGLFont#DEL_CHAR} is also not rendered
	 */
	public static final int CHAR_MIN = 32, CHAR_MAX = 256; // 0-31 are control codes
	
	/**
	 * This character is ignored because it has no visual representation
	 */
	public static final int DEL_CHAR = 127;
	
	/**
	 * Bitmap image of each character CHAR_MIN - CHAR_MAX excluding the DEL character
	 */
	private Texture fontTexture;
	
	/**
	 * The font to generate everything from
	 */
	private Font font;
	
	/**
	 * Useful information about the font
	 */
	private FontMetrics metrics;
	
	/**
	 * The starting pixel position in the {@link OpenGLFont#fontTexture} for each character
	 * The extra space is for the width of the image
	 */
	private int[] offsets = new int[CHAR_MAX - CHAR_MIN + 1];
	
	/**
	 * The height of each character of the font
	 */
	private int fontHeight;
	
	/**
	 * Generates the information about the font needed to get a characters width/height
	 * @param f The font to pull them from
	 * @return The FontMetrics the font generates
	 */
	private static FontMetrics getMetrics(@Nonnull Font f)
	{
		BufferedImage image = new BufferedImage(1, 1, BufferedImage.TYPE_INT_ARGB);
		Graphics2D g = image.createGraphics();
		
		g.setFont(f);
		FontMetrics metrics = g.getFontMetrics();
		g.dispose();
		
		return metrics;
	}
	
	/**
	 * <p>A way of representing a drawable font using opengl</p>
	 * <p>Use {@link OpenGLFont#bind()} to start using it, and {@link OpenGLFont#unbind()} to stop.</p>
	 * <p>This will unbind any texture currently being used</p>
	 * @param f The java font to generate this from
	 */
	public OpenGLFont(@Nonnull Font f)
	{
		font = f;
		metrics = getMetrics(f);
	}
	
	/**
	 * Initializes the font information onto the graphics card
	 */
	public void init()
	{
		int imgWidth = 0;
		
		fontHeight = metrics.getHeight();
		
		for(int i = CHAR_MIN; i <= CHAR_MAX; i++)
		{
			if(i == DEL_CHAR)
				continue;
			
			char c = (char)i;
			
			offsets[i - CHAR_MIN] = imgWidth;
			
			imgWidth += metrics.charWidth(c);
		}
		
		BufferedImage image = new BufferedImage(imgWidth, fontHeight, BufferedImage.TYPE_INT_ARGB);
		Graphics2D g = image.createGraphics();
		
		offsets[CHAR_MAX - CHAR_MIN] = imgWidth;
		
		g.setPaint(Color.WHITE);
		
		g.setFont(font);

		for(int i = CHAR_MIN; i <= CHAR_MAX; i++)
		{
			if(i == DEL_CHAR)
				continue;
			
			char c = (char)i;
			
			g.drawString(String.valueOf(c), getOffset(c), metrics.getAscent());
		}
		
		g.dispose();
		
		fontTexture = Texture.loadTexture(image);
	}
	
	/**
	 * {@link Texture#unbind()}
	 */
	public static void unbind()
	{
		Texture.unbind();
	}

	/**
	 * {@link Texture#bind()}
	 */
	public void bind()
	{
		fontTexture.bind();
	}
	
	/**
	 * Finds the u coordinate in the u,v texture pair for a given character - used to draw a character on a mesh.
	 * @param c The character to get the u texture coordinate for
	 * @return The u coordinate in the u,v texture pair for a given character
	 */
	public float uBegin(char c)
	{
		return getOffset(c) / (float)offsets[offsets.length - 1];
	}
	
	/**
	 * Finds the u ending coordinate in the u,v texture pair for a given character - used to draw a character on a mesh.
	 * @param c The character to get the u ending texture coordinate for
	 * @return The u ending coordinate in the u,v texture pair for a given character
	 */
	public float uEnd(char c)
	{
		return getOffset((char)((int)c + 1)) / (float)offsets[offsets.length - 1];
	}
	
	/**
	 * Gets a character's width in pixels
	 * @param c The character to get the width of
	 * @return a character's width in pixels
	 */
	public int charWidth(char c)
	{
		return getOffset((char)((int)c + 1)) - getOffset(c);
	}
	
	/**
	 * Gets the offset in pixels for a character in the font image
	 * @param c The character to do this for
	 * @return the offset in pixels for a character in the font image
	 */
	private int getOffset(char c)
	{
		return offsets[(int)c - CHAR_MIN];
	}
	
	/**
	 * The font this is generated from.
	 * Changing this font after calling {@link OpenGLFont#init()} will do nothing
	 * @return The font this is generated from
	 */
	public Font font() { return font; }
	
	/**
	 * The height of the font in pixels
	 * @return height of the font in pixels
	 */
	public int height() { return fontHeight; }
}
