package com.cornchipss.cosmos.world;

import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;

import org.joml.Matrix4f;
import org.joml.Vector3i;
import org.joml.Vector3ic;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.blocks.Blocks;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.io.IWritable;
	
public class Chunk implements IWritable
{
	/**
	 * Dimensions of a Chunk - must be even
	 */
	public static final int WIDTH = 16, HEIGHT = 16, LENGTH = 16;
	
	private Block[][][] blocks;
	public Block[][][] blocks() { return blocks; }
	
	private Vector3ic structureIndex;
	
	/**
	 * Offset relative to block structure's 0,0,0
	 */
	private Vector3ic offset;
	
	/**
	 * block structure it's a part of
	 */
	private Structure structure;
	
	/**
	 * Neighbors
	 */
	private Chunk left, right, top, bottom, front, back;
	
	public Chunk(int offX, int offY, int offZ, Vector3i structureIndex, Structure s)
	{
		this.offset = new Vector3i(offX, offY, offZ);
		
		this.structure = s;
		this.structureIndex = structureIndex;
		
		blocks = createBlocksArray(WIDTH, HEIGHT, LENGTH);
	}
	
	protected Block[][][] createBlocksArray(int w, int h, int l)
	{
		return new Block[l][w][h];
	}
	
	@Override
	public void write(DataOutputStream writer) throws IOException
	{
		short currentId = -1;
		int amount = 0;
		
		for(int z = 0; z < blocks.length; z++)
		{
			for(int y = 0; y < blocks[z].length; y++)
			{
				for(int x = 0; x < blocks[z][y].length; x++)
				{
					short id = blocks[z][y][x] != null ? blocks[z][y][x].numericId() : 0;
					
					if(currentId == -1)
					{
						currentId = id;
						amount = 1;
					}
					else if(currentId != id)
					{
						writer.writeShort(currentId);
						writer.writeInt(amount);
						
						currentId = id;
						amount = 1;
					}
					else
					{
						amount++;
					}
				}
			}
		}
		
		// This would only ever be true if the chunk was 0x0x0.
		if(currentId != -1)
		{
			writer.writeShort(currentId);
			writer.writeInt(amount);
		}
	}
	
	@Override
	public void read(DataInputStream reader) throws IOException
	{
		int position = 0;
		
		while(position < WIDTH * HEIGHT * LENGTH)
		{
			short id = reader.readShort();
			int amount = reader.readInt();
			
			for(int i = 0; i < amount; i++)
			{
				int z = position / HEIGHT / WIDTH;
				int y = (position - z * HEIGHT * WIDTH) / HEIGHT;
				int x = position - z * HEIGHT * WIDTH - y * HEIGHT;
				
				if(id != 0) 
				
				block(x, y, z, Blocks.fromNumericId(id));
				
				position++;
			}
		}
	}
	
	public void leftNeighbor(Chunk c)
	{
		left = c;
	}
	public void rightNeighbor(Chunk c)
	{
		right = c;
	}
	public void topNeighbor(Chunk c)
	{
		top = c;
	}
	public void bottomNeighbor(Chunk c)
	{
		bottom = c;
	}
	public void frontNeighbor(Chunk c)
	{
		front = c;
	}
	public void backNeighbor(Chunk c)
	{
		back = c;
	}
	
	public Chunk leftNeighbor()
	{
		return left;
	}
	public Chunk rightNeighbor()
	{
		return right;
	}
	public Chunk topNeighbor()
	{
		return top;
	}
	public Chunk bottomNeighbor()
	{
		return bottom;
	}
	public Chunk frontNeighbor()
	{
		return front;
	}
	public Chunk backNeighbor()
	{
		return back;
	}
	
	private Matrix4f transformMatrix;

	public void block(int x, int y, int z, Block block)
	{
		block(x, y, z, block, true);
	}
	
	public boolean within(int x, int y, int z)
	{
		return x >= 0 && x < WIDTH &&
				y >= 0 && y < HEIGHT &&
				z >= 0 && z < LENGTH;
	}
	
	/**
	 * <p>Sets the block at the given coordinates relative to this chunk.</p>
	 * <p>If {@link Chunk#render()} has been called previously, this will also call it.</p>
	 * @param x The X coordinate relative to this chunk
	 * @param y The Y coordinate relative to this chunk
	 * @param z The Z coordinate relative to this chunk
	 * @param block The block to set it to
	 * @param shouldRender If true, the chunk will update the lightmap + re-render the mesh. If the chunk has not been rendered yet - this will be false no matter what is passed
	 */
	public void block(int x, int y, int z, Block block, boolean shouldRender)
	{	
		if(!within(x, y, z))
			throw new IllegalArgumentException("Bad x,y,z: " + x + ", " + y + ", " + z);
		
		blocks[z][y][x] = block;
	}
	
	
	/**
	 * Gets the block relative to this chunk's position
	 * @param x The X coordinate relative to the chunk's position
	 * @param y The Y coordinate relative to the chunk's position
	 * @param z The Z coordinate relative to the chunk's position
	 * @return The block at this position - null if there is no block
	 */
	public Block block(int x, int y, int z)
	{
		return blocks[z][y][x];
	}
	
	public int width() { return WIDTH; }
	public int height() { return HEIGHT; }
	public int length() { return LENGTH; }

	public void transformMatrix(Matrix4f m)
	{
		transformMatrix = m;
	}
	
	public Matrix4f transformMatrix()
	{
		return transformMatrix;
	}

	public Vector3ic offset()
	{
		return offset;
	}
	
	public Structure structure()
	{
		return structure;
	}

	public Vector3ic index()
	{
		return structureIndex;
	}
}
