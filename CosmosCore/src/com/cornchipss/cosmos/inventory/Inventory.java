package com.cornchipss.cosmos.inventory;

import com.cornchipss.cosmos.blocks.Block;

public class Inventory
{
	private Block[][] blocks;
	
	private int rows, cols;
	
	public Inventory(int rows, int cols)
	{
		this.rows = rows;
		this.cols = cols;
		blocks = new Block[rows][cols];
	}
	
	public int rows()
	{
		return rows;
	}
	
	public int columns()
	{
		return cols;
	}
	
	public Block block(int row, int col)
	{
		return blocks[row][col];
	}
	
	public void block(int row, int col, Block b)
	{
		blocks[row][col] = b;
	}
}
