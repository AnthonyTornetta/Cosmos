# Identifiable

## Intent

The **Identifiable** trait provides an easy way to mark a piece of data as unique from others via an unlocalized name &amp; numeric id combination.

### Unlocalized Name

The unlocalized name is a user-defined constant that will serve as that piece of data's unique identifier for its lifetime. Unlocalized names should always be formated as `mod_id:item_name`. This makes sure that name collisions don't happen across multiple mods. Cosmos uses the mod id of `cosmos`. For example, the grass block has an unlocalized name of `cosmos:grass`.

These are used everywhere, both inside the game code and in assets such as the lang files and textures. These should remain constant across version changes to reduce breaking on any third-party dependencies.

### Numeric Ids

This is set by the [`Registry`](./index.md) whenever an `Identifiable` type is registered, so its initial value before `set_numeric_id` is called does not matter. These are used internally by registries and should be generally avoided because they may change as new blocks/items/etc are added or old ones are removed. If you serialize numeric ids at any point, make sure you provide a mapping to their unlocalized name counterparts to ensure it can be deserialized without breaking if new items are added or removed.

These only exist speed and a lower memory usage.

## Example

```rs
struct LaserType {
    id: u16,
    unlocalized_name: String,
    speed: f32,
}

impl Identifiable for LaserType {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

fn add_lasers(mut laser_registry: ResMut<Registry<LaserType>>) {
    let fast_laser = LaserType {
        id: 0, // initial value doesn't matter - will be overwritten by the registry via `set_numeric_id`
        unlocalized_name: "cosmos:fast_laser".to_owned(),
        speed: 100.0,
    };

    let slow_laser = LaserType {
        id: 0,
        unlocalized_name: "cosmos:slow_laser".to_owned(),
        speed: 10.0,
    };

    laser_registry.register(fast_laser); // fast_laser id will now be 0
    laser_registry.register(slow_laser); // slow_laser id will now be 1
}

pub(super) fn register(app: &mut App) {
    registry::create_registry::<LaserType>(app);

    app.add_systems(OnEnter(GameState::Loading), add_lasers);
}
```