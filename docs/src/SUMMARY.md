<!--
The first time you view the cosmos documentation, run the following commands

cargo install mdbook
cargo install mdbook-mermaid

Every time you want to view the documentation, navigate to the `docs/` directory.

To view the documentation, navigate to the `docs/` directory. To have it update as you modify it,
run `mdbook serve` and navigate to the URL it provides, or to just build it run `mdbook build`.
 -->

<!--
For mermaid js syntax, see here: https://mermaid.js.org/syntax/

The mermaid markdown syntax highlighting + markdown preview mermaid plugins are nice to have for VS code
-->

- [Cosmos](./index.md)
- [Registries](./registries/index.md)
  - [Identifiable](./registries/identifiable.md)
  - [Many to One Registry](./registries/many_to_one_registry.md)
- [Biomes](./biomes/index.md)
- [Physics](./physics/index.md)
  - [Location](./physics/location.md)
  - [Anchor](./physics/anchor.md)
- [Packets](./packets/index.md)
  - [Player movement](./packets/player-movement.md)
  - [Updating bodies of entities](./packets/bulk-bodies.md)
