# Cosmos

## Release 0.0.1a
- Player Movement
  - FPS Camera
- Dynamic Input system
- Structure
  - Planet
  - Ship
- Asset Loading
  - Fix UV mapping floating point errors
- Planet Generation
  - Grass planet generator
- Server/Client
  - Server controls planet
  - Each player controls its own movement (100% trusted)
- Ship creation
- Block registration
  - Block registry with numeric + fixed string IDs
- Player entity
- Game State
- Network communication
- Sync bodies from server to client
- Ability to break/replace block
  - Dynamic meshing
  - Dynamic physics bodies
  - Block break events sent to server, server sends back block changed event
  - Selects nearest structure
- Integrate physics system
  - Physics generator for structures
    - Chunk-based physics
  - Player collider
- Integrate bevy engine
  - Rendering method for structures
    - Chunk-based rendering
- Add Crosshair
- Support re-sizable window

## Release 0.0.2a

// todo