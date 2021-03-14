package com.cornchipss.cosmos.material;

public class Materials
{
	public static final Material
		DEFAULT_MATERIAL = new DefaultMaterial(),
		GUI_MATERIAL = new GuiMaterial(),
		ANIMATED_DEFAULT_MATERIAL = new AnimatedDefaultMaterial();
	
	public static void initMaterials()
	{
		DEFAULT_MATERIAL.init();
		
		ANIMATED_DEFAULT_MATERIAL.init();

		GUI_MATERIAL.init();
	}
}
