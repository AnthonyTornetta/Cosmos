package com.cornchipss.cosmos.game;

import java.awt.Font;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;

import org.joml.Matrix4f;
import org.joml.Matrix4fc;
import org.joml.Vector3f;
import org.lwjgl.glfw.GLFW;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL13;
import org.lwjgl.opengl.GL30;

import com.cornchipss.cosmos.biospheres.Biosphere;
import com.cornchipss.cosmos.blocks.Blocks;
import com.cornchipss.cosmos.blocks.ClientBlock;
import com.cornchipss.cosmos.gui.GUI;
import com.cornchipss.cosmos.gui.GUIModel;
import com.cornchipss.cosmos.gui.GUITexture;
import com.cornchipss.cosmos.gui.GUITextureMultiple;
import com.cornchipss.cosmos.gui.text.GUIText;
import com.cornchipss.cosmos.gui.text.OpenGLFont;
import com.cornchipss.cosmos.material.Materials;
import com.cornchipss.cosmos.physx.Transform;
import com.cornchipss.cosmos.registry.Biospheres;
import com.cornchipss.cosmos.rendering.MaterialMesh;
import com.cornchipss.cosmos.rendering.Window;
import com.cornchipss.cosmos.structures.Planet;
import com.cornchipss.cosmos.structures.Ship;
import com.cornchipss.cosmos.structures.Structure;
import com.cornchipss.cosmos.utils.DebugMonitor;
import com.cornchipss.cosmos.utils.io.Input;
import com.cornchipss.cosmos.world.Chunk;
import com.cornchipss.cosmos.world.entities.player.ClientPlayer;

public class ClientGame extends Game
{
	private Planet mainPlanet;
	private Ship ship;
	private Matrix4f projectionMatrix;
	private ClientPlayer p;
	private GUI gui;
	private int selectedSlot;
	private GUITextureMultiple[] inventorySlots;
	private GUIText fpsText;
	
	public ClientGame(Window window)
	{
		gui = new GUI(Materials.GUI_MATERIAL);
		gui.init(window.getWidth(), window.getHeight());
		
		GUITexture crosshair = new GUITexture(new Vector3f(window.getWidth() / 2.f - 16, window.getHeight() / 2.f - 16, 0), 32, 32, 0, 0);
		gui.addElement(crosshair);
		
		OpenGLFont font = new OpenGLFont(new Font("Arial", Font.PLAIN, 28));
		font.init();
		
		inventorySlots = new GUITextureMultiple[10];
		
		GUIModel[] models = new GUIModel[10];
		
		p = new ClientPlayer(world());
		p.addToWorld(new Transform(0, 0, 0));
		
		int slotDimensions = 64;
		
		int startX = (int)(1024 / 2.0f - (inventorySlots.length / 2.0f) * slotDimensions);
		
		for(int i = 0; i < models.length; i++)
		{
			inventorySlots[i] =  new GUITextureMultiple(
					new Vector3f(startX + i * slotDimensions, 0, 0), slotDimensions, slotDimensions, 
					0.5f, 0,
					0, 0.5f);
			
			gui.addElement(inventorySlots[i]);
			
			if(i < p.inventory().columns() && p.inventory().block(0, i) != null)
			{
				int margin = 4;
				
				models[i] = new GUIModel(
						new Vector3f(startX + i * slotDimensions + margin, margin, 0), 
						slotDimensions - margin * 2, 
						((ClientBlock)(p.inventory().block(0, i))).model());
				
				gui.addElement(models[i]);
			}
		}
		
		inventorySlots[selectedSlot].state(1);
		
		fpsText = new GUIText("-- --ms", font, 0, 0);
		gui.addElement(fpsText);
		
		mainPlanet = new Planet(world(), 16*10, 16*6, 16*10);
		mainPlanet.init();
		Biosphere def = Biospheres.newInstance("cosmos:desert");
		def.generatePlanet(mainPlanet);
		world().addStructure(mainPlanet);
		
		ship = new Ship(world());
		ship.init();
		world().addStructure(ship);
		
		try(DataInputStream shipStr = new DataInputStream(new FileInputStream(new File("assets/structures/ships/test.struct"))))
		{
			ship.read(shipStr);
		}
		catch(IOException ex)
		{
			ex.printStackTrace();
			ship.block(ship.width() / 2, ship.height() / 2, ship.length() / 2, Blocks.SHIP_CORE);
		}
		
		ship.addToWorld(new Transform());
		
		ship.calculateLights(false);
		
		for(Chunk c : ship.chunks())
			c.render();
		
		mainPlanet.calculateLights(false);
		
		for(Chunk c : mainPlanet.chunks())
			c.render();
		
		mainPlanet.addToWorld(new Transform(0, -mainPlanet.height(), 0));
		
		projectionMatrix = new Matrix4f();
		projectionMatrix.perspective((float)Math.toRadians(90), 
				1024/720.0f,
				0.1f, 1000);
		
		Matrix4f guiProjMatrix = new Matrix4f();
		guiProjMatrix.perspective((float)Math.toRadians(90), 
				1024/720.0f,
				0.1f, 1000);
	}
	
	public void onResize(int w, int h)
	{
		projectionMatrix.identity();
		projectionMatrix.perspective((float)Math.toRadians(90), 
				w/(float)h,
				0.1f, 1000);
		
		gui.updateProjection(w, h);
	}
	
	public void render(float delta)
	{
		GL11.glEnable(GL13.GL_TEXTURE0);
		
		GL30.glEnable(GL30.GL_DEPTH_TEST);
		GL30.glDepthFunc(GL30.GL_LESS);
		
		//GL30.glPolygonMode(GL30.GL_FRONT_AND_BACK, GL30.GL_LINE);
		
		for(Structure s : world().structures())
		{
			drawStructure(s, projectionMatrix, p);
		}
		
		gui.draw();
	}
	
	@Override
	public void update(float delta)
	{
		super.update(delta);
		
		fpsText.text(DebugMonitor.get("ups") + " " + (int)((Float)DebugMonitor.get("ups-variance")*1000) + "ms");
		
		int prevRow = p.selectedInventoryColumn();
		
		p.update(delta);
		
		int row = p.selectedInventoryColumn();
		
		if(prevRow != row)
		{
			inventorySlots[prevRow].state(0);
			inventorySlots[row].state(1);
		}
		
		if(Input.isKeyJustDown(GLFW.GLFW_KEY_ENTER))
		{
			try(DataOutputStream str = new DataOutputStream(new FileOutputStream(new File("assets/structures/ships/test.struct"))))
			{
				ship.write(str);
			}
			catch(IOException ex)
			{
				ex.printStackTrace();
			}
		}
	}

	private static void drawStructure(Structure s, Matrix4fc projectionMatrix, ClientPlayer p)
	{
		for(Chunk chunk : s.chunks())
		{
			Matrix4f transform = new Matrix4f();
			Matrix4fc trans = s.openGLMatrix();
			trans.mul(chunk.transformMatrix(), transform);
			
			for(MaterialMesh m : chunk.model().materialMeshes())
			{
				m.material().use();
				
				Matrix4fc camera = p.shipPiloting() == null ? 
						p.camera().viewMatrix() : 
							p.shipPiloting().body().transform().invertedMatrix();
				
				m.material().initUniforms(projectionMatrix, camera, transform, false);
				
				m.mesh().prepare();
				m.mesh().draw();
				m.mesh().finish();
				
				m.material().stop();
			}
		}
	}
}
