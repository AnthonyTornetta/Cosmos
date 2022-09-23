# CosmosRust
Cosmos, but in rust

# Cosmos Roadmap

## Release 0.0.1a
- [x] Player Movement
  - [x] FPS Camera
- [x] Dynamic Input system
- [x] Structure
  - [x] Planet
  - [x] Ship
- [x] Asset Loading
  - [x] Fix UV mapping floating point errors
- [x] Planet Generation
  - [x] Grass planet generator
- [x] Server/Client
  - [x] Server controls planet
  - [x] Each player controls its own movement (100% trusted)
- [x] Ship creation
- [x] Block registration
  - [x] Block registry with numeric + fixed string IDs
- [x] Player entity
- [x] Game State
- [x] Network communication
- [x] Sync bodies from server to client
- [x] Ability to break/replace block
  - [x] Dynamic meshing
  - [x] Dynamic physics bodies
  - [x] Block break events sent to server, server sends back block changed event
  - [x] Selects nearest structure
- [x] Integrate physics system
  - [x] Physics generator for structures
    - [x] Chunk-based physics
  - [x] Player collider
- [x] Integrate bevy engine
  - [x] Rendering method for structures
    - [x] Chunk-based rendering
- [x] Add Crosshair
- [x] Support re-sizable window

## Release 0.0.2a

// todo