# Physics

Cosmos uses a custom fork of [bevy_rapier](https://github.com/AnthonyTornetta/bevy_rapier) as the physics backend. This fork supports multiple independent physics worlds, better behavior of child/parent relationships, and various bug fixes.

Due to the near-infinite universe Cosmos takes place in, the [Location](./location.md) structure is used to represent a point in space instead of a transform. Transforms now represent your position relative to a world's [anchor](./anchor.md).