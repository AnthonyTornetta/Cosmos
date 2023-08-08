# Registries

## Intent

**Registries** are a way of storing groups of data that can be added &amp; removed at runtime.

## :( Problem

There are many cases where we want to group pieces of data together in a way where items can be dynamically added or removed. For example, the blocks of cosmos are all created when the game starts up, and can be created in any number of different areas. Storing each block as a static constant would make this impossible. You would have to know every block that would be in the game at compile-time, which becomes impossible once external modifications to the game are supported.

## :) Solution

Rather than storing items as constant static values, a **Registry** is created. A Registry would store a key-value pairing for its contents. The key would have to be something unique to the specific item it's storing, which is referred to as its [unlocalized name](./identifiable.md). 

Most things that are dynamically added to the game (such as blocks, items, materials, textures) require their own registry. If an item is not within a registry, Cosmos has no way of knowing that data exists.

All items must have a unique [unlocalized name](./identifiable.md) to be in a registry. Note that two different registries may have the same unlocalized names. For example, the `Registry<Block>` and `Registry<Item>` could have the same key used in both to represent two different things. If you add two items with the same unlocalized name to the same registry, the second addition will overwrite the previous entry.

There are two registries commonly used - the basic registry known as the `Registry` type and the [many to one registry](./many_to_one_registry.md) known as the `ManyToOneRegistry` type. The basic registry simply maps a key to its value, as shown below. The Many to One registry maps many identifiable types to the same identifiable value.

Registries are designed to be generic across any types of data, so to create a registry for a specific type it must implement the [`Identifiable`](./identifiable.md) trait.

## Examples

### Creating a `Block` registry and adding blocks to it

```rs
fn create_blocks(mut block_registry: ResMut<Registry<Block>>) {
    block_registry.register(
        BlockBuilder::new("cosmos:stone", 10.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );

    block_registry.register(
        BlockBuilder::new("cosmos:grass", 3.0)
            .add_property(BlockProperty::Opaque)
            .add_property(BlockProperty::Full)
            .create(),
    );
}

fn print_blocks(block_registry: Res<Registry<Block>>) {
    // Easily iterate over every item of a registry
    for block in block_registry.iter() {
        println!("This block exists: {}", block.unlocalized_name());
    }

    // Use `Registry::from_id` to get an item stored at that key
    println!("Grass exists: {}", block_registry.from_id("cosmos:grass").is_some());
}

pub(super) fn register(app: &mut App) {
    // Sets up the registry for use as a resource of type Registry<Block>
    registry::create_registry<Block>(app);

    // Blocks are commonly loaded in the GameState::Loading state, 
    // but a registry is available in any state up its creation.
    app.add_systems(OnEnter(GameState::Loading), create_blocks)
        .add_systems(OnExit(GameState::Loading), print_blocks);
}
```