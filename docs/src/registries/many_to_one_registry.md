# Many to One Registry

## Intent
The **ManyToOneRegistry** provides a way of linking multiple identifiable instances with one identifiable instance.

## Problem :(
A normal registry is great for modeling 1:1 relationships. You can simply use the same unlocalized name for the key of one registry with another. For example,  the blocks registry. However, when trying to model many identifiable items to one identifiable items, sharing an unlocalized name is no longer an option. This is a problem for sharing things such as models across multiple items or blocks.

## Solution :)
The **ManyToOneRegistry** solves this problem by allowing you to map multiple identifiable objects to the same value. To use this registry, simply add the value first, then link as many keys to it as needed.

### Example
In this example, multiple blocks point to the same model information

```rs
fn fill_registry(blocks_registry: Res<Registry<Block>>, mut registry: ResMut<ManyToOneRegistry<Block, BlockMeshInformation>>) {
    // Add value to the registry
    registry.insert_value(BlockMeshInformation::new_single_mesh_info(
        "cosmos:cube_model",
        MeshInformation::default(), // this does not actually create a cube
    ));

    // Add all the links to that previously added value
    for block in blocks_registry.iter() {
        if !registry.contains(block) {
            registry
                .add_link(block, "cosmos:cube_model")
                .expect("The cosmos:cube_model was added above, so this will never fail.");
        }
    }

    // Get a value via one of the many keys
    if let Some(grass_block) = blocks_registry.from_id("cosmos:grass") {
        // This will print out the BlockMeshInformation added above
        println!("Grass model: {:?}", registry.get_value(grass_block));
    }
}

pub(super) fn register(app: &mut App) {
    many_to_one::create_many_to_one_registry::<Block, BlockMeshInformation>(app);

    app.add_systems(OnEnter(GameState::PostLoading), fill_registry);
}
```