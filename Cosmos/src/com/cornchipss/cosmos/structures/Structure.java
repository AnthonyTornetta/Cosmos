package com.cornchipss.cosmos.structures;

import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.util.LinkedHashSet;
import java.util.Set;

import org.joml.Matrix4f;
import org.joml.Matrix4fc;
import org.joml.Vector3f;
import org.joml.Vector3fc;
import org.joml.Vector3i;
import org.joml.Vector3ic;
import org.joml.Vector4f;

import com.cornchipss.cosmos.blocks.Block;
import com.cornchipss.cosmos.lights.LightMap;
import com.cornchipss.cosmos.physx.PhysicalObject;
import com.cornchipss.cosmos.physx.RigidBody;
import com.cornchipss.cosmos.physx.StructureShape;
import com.cornchipss.cosmos.physx.Transform;
import com.cornchipss.cosmos.utils.Logger;
import com.cornchipss.cosmos.utils.Maths;
import com.cornchipss.cosmos.utils.io.IWritable;
import com.cornchipss.cosmos.world.Chunk;
import com.cornchipss.cosmos.world.World;

public abstract class Structure extends PhysicalObject implements IWritable
{
	private Chunk[] chunks;
	
	private boolean initialized = false;
	
	private int width, height, length;
	
	private int cWidth, cHeight, cLength;
	
	private LightMap lightMap;
	
	private StructureShape shape;
	
	public StructureShape shape() { return shape; }
	
	private Set<Chunk> bulkUpdate;
	
	public Structure(World world)
	{
		super(world);
	}
	
	public Structure(World world, int width, int height, int length)
	{
		super(world);
		
		if(width <= 0 || height <= 0 || length <= 0)
			throw new IllegalArgumentException("A Structure's width/height/length cannot be <= 0");
		
		this.width = width;
		this.height = height;
		this.length = length;
		
		cLength = (int)Math.ceil((float)length / Chunk.LENGTH);
		cHeight = (int)Math.ceil((float)height / Chunk.HEIGHT);
		cWidth = (int)Math.ceil((float)width / Chunk.WIDTH);
		
		lightMap = new LightMap(width + 2, height + 2, length + 2);
		
		chunks = new Chunk[cLength * cHeight * cWidth];
		
		shape = new StructureShape(this);
		
		bulkUpdate = null;
	}
	
	/**
	 * For sub classes to override if needed
	 */
	public void update(float delta)
	{
		
	}
	
	public void explode(int radius, Vector3i pos)
	{
		beginBulkUpdate();
		
		Vector3f temp = new Vector3f();
		Vector3f tempPos = new Vector3f(pos.x, pos.y, pos.z);
		
		for(int dz = -radius; dz <= radius; dz++)
		{
			for(int dy = -radius; dy <= radius; dy++)
			{
				for(int dx = -radius; dx <= radius; dx++)
				{									
					int xx = pos.x + dx,
						yy = pos.y + dy, 
						zz = pos.z + dz;
					
					temp.x = xx;
					temp.y = yy;
					temp.z = zz;
					
					if(Maths.distSqrd(temp, tempPos) < radius * radius)
					{
						if(withinBlocks(xx, yy, zz))
						{
							block(xx, yy, zz, null);
						}
					}
				}
			}
		}
		
		endBulkUpdate();
	}
	
	public boolean bulkUpdating()
	{
		return bulkUpdate != null;
	}

	public void beginBulkUpdate()
	{
		if(!bulkUpdating())
			bulkUpdate = new LinkedHashSet<>();
	}

	public void endBulkUpdate()
	{
		if(bulkUpdating())
		{
			calculateLights(true);
			
			Set<Chunk> all = new LinkedHashSet<>();
		
			for(Chunk c : bulkUpdate)
			{
				// not the best way for account for all changes, given that large light sources would invalidate this, but it works for now
				all.add(c);
				if(c.leftNeighbor() != null)
					all.add(c.leftNeighbor());
				if(c.rightNeighbor() != null)
					all.add(c.rightNeighbor());
				if(c.topNeighbor() != null)
					all.add(c.topNeighbor());
				if(c.bottomNeighbor() != null)
					all.add(c.bottomNeighbor());
				if(c.frontNeighbor() != null)
					all.add(c.frontNeighbor());
				if(c.backNeighbor() != null)
					all.add(c.backNeighbor());
			}
			
			for(Chunk c : all)
				c.render();
		}
		else
			throw new IllegalStateException("Cannot end a bulk update when there is no bulk update currently happening");
		
		bulkUpdate = null;
	}
	
	@Override
	public void addToWorld(Transform transform)
	{
		body(new RigidBody(transform));
		world().addRigidBody(body());
	}
	
	public int chunksLength()
	{
		return cLength;
	}
	public int chunksHeight()
	{
		return cHeight;
	}
	public int chunksWidth()
	{
		return cWidth;
	}
	
	public Chunk[] chunks()
	{
		return chunks;
	}
	
	private int flatten(int x, int y, int z)
	{
		if(!within(x, y, z))
			throw new IndexOutOfBoundsException(x + "," + y + "," + z + " is out of bounds for " + cWidth + "x" + cHeight + "x" + cLength + ".");
		return x + cWidth * (y + cHeight * z);
	}
	
	private void chunkAt(int x, int y, int z, Chunk c)
	{
		chunks[flatten(x, y, z)] = c;
	}
	
	private Chunk chunkAt(int x, int y, int z)
	{
		return chunks[flatten(x, y, z)];
	}
	
	public Chunk chunk(int x, int y, int z)
	{
		return chunkAt(x / Chunk.WIDTH, y / Chunk.HEIGHT, z / Chunk.LENGTH);
	}
	
	public void chunk(int x, int y, int z, Chunk c)
	{
		chunkAt(x / Chunk.WIDTH, y / Chunk.HEIGHT, z / Chunk.LENGTH, c);
	}
	
	public void init()
	{
		initialized = true;
		
		for(int z = 0; z < chunksLength(); z++)
		{
			for(int y = 0; y < chunksHeight(); y++)
			{
				for(int x = 0; x < chunksWidth(); x++)
				{
					int i = flatten(x, y, z);
					chunks[i] = new Chunk(x * Chunk.WIDTH + 1, y * Chunk.HEIGHT + 1, z * Chunk.LENGTH + 1, this);
					
					chunks[i].transformMatrix(
							Maths.createTransformationMatrix(
									new Vector3f(
											x * Chunk.WIDTH, 
											y * Chunk.HEIGHT, 
											z * Chunk.LENGTH), 
									Maths.blankQuaternion()));

				}
			}
		}
		
		for(int z = 0; z < chunksLength(); z++)
		{
			for(int y = 0; y < chunksHeight(); y++)
			{
				for(int x = 0; x < chunksWidth(); x++)
				{
					int i = flatten(x, y, z);
					
					chunks[i].leftNeighbor(
							within(x - 1, y, z) ? chunks[flatten(x - 1, y, z)] : null);
					chunks[i].rightNeighbor(
							within(x + 1, y, z) ? chunks[flatten(x + 1, y, z)] : null);
					chunks[i].topNeighbor(
							within(x, y + 1, z) ? chunks[flatten(x, y + 1, z)] : null);
					chunks[i].bottomNeighbor(
							within(x, y - 1, z) ? chunks[flatten(x, y - 1, z)] : null);
					chunks[i].frontNeighbor(
							within(x, y, z + 1) ? chunks[flatten(x, y, z + 1)] : null);
					chunks[i].backNeighbor(
							within(x, y, z - 1) ? chunks[flatten(x, y, z - 1)] : null);
					
				}
			}
		}
	}
	
	public void calculateLights(boolean render)
	{
		long start = System.currentTimeMillis();
		
		Vector3i[] changedArea = lightMap.calculateLightMap();
		
		long end = System.currentTimeMillis();
		
		Logger.LOGGER.debug(end - start + "ms to calculate light map");
		
		if(render)
		{
			Vector3i extremeNeg = changedArea[0];
			Vector3i extremePos = changedArea[1];
			
			if(extremeNeg.x() != -1) // if it isn't -1, then none of them are negative 1
			{
				// TODO: fix this, for some reason the extremeNeg + Pos calcs don't work. Idk why
				extremeNeg.x = Maths.min(extremeNeg.x - Chunk.WIDTH, 0);
				extremeNeg.y = Maths.min(extremeNeg.y - Chunk.HEIGHT, 0);
				extremeNeg.z = Maths.min(extremeNeg.z - Chunk.LENGTH, 0);
				
				extremePos.x = Maths.min(extremePos.x + Chunk.WIDTH, width());
				extremePos.y = Maths.min(extremePos.y + Chunk.HEIGHT, height());
				extremePos.z = Maths.min(extremePos.z + Chunk.LENGTH, length());
				
				// Account for the +2 size of the light map
				extremeNeg.x += 1;
				extremeNeg.y += 1;
				extremeNeg.z += 1;
				
				extremePos.x -= 1;
				extremePos.y -= 1;
				extremePos.z -= 1;
				
				for(int cz = extremeNeg.z() / 16; cz < Math.ceil(extremePos.z() / 16.0f); cz++)
				{
					for(int cy = extremeNeg.y() / 16; cy < Math.ceil(extremePos.y() / 16.0f); cy++)
					{
						for(int cx = extremeNeg.x() / 16; cx < Math.ceil(extremePos.x() / 16.0f); cx++)
						{
							chunks[flatten(cx, cy, cz)].render();
						}
					}
				}
			}
		}
	}
	
	@Override
	public void write(DataOutputStream writer) throws IOException
	{
		long sec = System.currentTimeMillis();
		
		writer.writeInt(width);
		writer.writeInt(height);
		writer.writeInt(length);
		
		for(int z = 0; z < chunksLength(); z++)
		{
			for(int y = 0; y < chunksHeight(); y++)
			{
				for(int x = 0; x < chunksWidth(); x++)
				{
					chunkAt(x, y, z).write(writer);
				}
			}
		}
		
		Logger.LOGGER.debug((System.currentTimeMillis() - sec) + "ms to save " + width() + "x" + height + "x" + length() + " structure.");
	}
	
	@Override
	public void read(DataInputStream reader) throws IOException
	{
		long sec = System.currentTimeMillis();
		
		this.width = reader.readInt();
		this.height = reader.readInt();
		this.length = reader.readInt();
		
		cLength = (int)Math.ceil((float)length / Chunk.LENGTH);
		cHeight = (int)Math.ceil((float)height / Chunk.HEIGHT);
		cWidth = (int)Math.ceil((float)width / Chunk.WIDTH);
		
		lightMap = new LightMap(width + 2, height + 2, length + 2);
		
		chunks = new Chunk[cLength * cHeight * cWidth];
		
		shape = new StructureShape(this);
		
		bulkUpdate = null;
		
		init();
		
		for(int z = 0; z < chunksLength(); z++)
		{
			for(int y = 0; y < chunksHeight(); y++)
			{
				for(int x = 0; x < chunksWidth(); x++)
				{
					chunkAt(x, y, z).read(reader);
				}
			}
		}
		
		Logger.LOGGER.debug((System.currentTimeMillis() - sec) + "ms to read " + width() + "x" + height + "x" + length() + " structure.");
	}
	
	public boolean within(int x, int y, int z)
	{
		return x >= 0 && x < cWidth && y >= 0 && y < cHeight && z >= 0 && z < cLength;
	}
	
	public boolean withinBlocks(int x, int y, int z)
	{
		return x >= 0 && x < width && y >= 0 && y < height && z >= 0 && z < length;
	}
	
	public void block(int x, int y, int z, Block b)
	{
		if(!initialized)
			init();
		
		if(withinBlocks(x, y, z))
		{
			Chunk c = chunk(x, y, z);
			
			c.block(x % Chunk.WIDTH, y % Chunk.HEIGHT, z % Chunk.LENGTH, b, !bulkUpdating());
			
			if(bulkUpdating())
				bulkUpdate.add(c);
		}
		else
			throw new IndexOutOfBoundsException(x + ", " + y + ", " + z + " was out of bounds for " + width + "x" + height + "x" + length);
	}
	
	public Block block(int x, int y, int z)
	{
		if(!initialized)
			init();
		
		if(withinBlocks(x, y, z))
		{
			Chunk c = chunk(x, y, z);
			
			return c.block(x % Chunk.WIDTH, y % Chunk.HEIGHT, z % Chunk.LENGTH);
		}
		else
			throw new IndexOutOfBoundsException(x + ", " + y + ", " + z + " was out of bounds for " + width + "x" + height + "x" + length);
	}
	
	public Vector3fc center()
	{
		return body().transform().position();
	}
	
	public int length() { return length; }
	public int height() { return height; }
	public int width() { return width; }

	/**
	 * Not implemented
	 * TODO implement this
	 * @param r
	 * @param x
	 * @param y
	 * @param z
	 */
	public void rotation(float r, float x, float y, float z)
	{
		throw new RuntimeException("Not yet implemented D;");
	}
	
	public Matrix4fc openGLMatrix()
	{
		Matrix4f mat = new Matrix4f();
		mat.set(body().transform().matrix());
		mat.translate(-width() / 2.f, -height() / 2.f, -length() / 2.f);
		return mat;
	}
	
	public Matrix4fc transformMatrix()
	{
		return body().transform().matrix();
	}
	
	public LightMap lightMap()
	{
		return lightMap;
	}

	public void removeBlock(int x, int y, int z)
	{
		block(x, y, z, null);
	}
	
	public Vector3i worldCoordsToStructureCoords(Vector3fc v)
	{
		return worldCoordsToStructureCoords(v.x(), v.y(), v.z());
	}
	
	public Vector3i worldCoordsToStructureCoords(Vector3ic v)
	{
		return worldCoordsToStructureCoords(v.x(), v.y(), v.z());
	}
	
	public Vector3i worldCoordsToStructureCoords(float x, float y, float z)
	{
		Vector4f c = new Vector4f(x, y, z, 1);
		
		body().transform().invertedMatrix().transform(c);
		
		return new Vector3i((int)c.x + width() / 2, (int)c.y + height() / 2, (int)c.z + length() / 2);
	}
	
	public Vector3f localCoordsToWorldCoords(float x, float y, float z, Vector3f storage)
	{
		Vector4f c = new Vector4f(x - width() / 2, y - height() / 2, z - length() / 2, 1);
		
		body().transform().matrix().transform(c);
		
		storage.set(c.x, c.y, c.z);
		
		return storage;
	}
	
	public Vector3f localCoordsToWorldCoords(float x, float y, float z)
	{
		return localCoordsToWorldCoords(x, y, z, new Vector3f());
	}
	
	public Vector3f localCoordsToWorldCoords(Vector3fc v, Vector3f storage)
	{
		return localCoordsToWorldCoords(v.x(), v.y(), v.z(), storage);
	}
	
	public Vector3f localCoordsToWorldCoords(Vector3fc v)
	{
		return localCoordsToWorldCoords(v.x(), v.y(), v.z());
	}
	
	public int higehstYAt(int x, int z)
	{
		for(int y = height() - 1; y >= 0; y--)
		{
			if(block(x, y, z) != null)
				return y;
		}
		
		return -1;
	}
	
	public int lowestYAt(int x, int z)
	{
		for(int y = 0; y < height(); y++)
		{
			if(block(x, y, z) != null)
				return y;
		}
		
		return -1;
	}

	public boolean hasBlock(int x, int y, int z)
	{
		return withinBlocks(x, y, z) && block(x, y, z) != null;
	}
}
