# Cosmos
A multiplayer block-based space exploration game, written in rust using the [Bevy](bevyengine.org/) engine.

## Compilation
This project requires the latest nightly rust to compile. To swap to nightly, run the command 

`rustup default nightly`.

To compile, navigate to the root directory of the repo and run 

`cargo build`.

To run the client, navigate to the cosmos_client directory and run

`cargo run`

For the server, navigate to the cosmos_server directory and run

`cargo run`

For release builds, append the `--release` flag to the build/run commands.

## Documentation

To view the cosmos documentation, run the following commands

```console
cargo install mdbook
cargo install mdbook-mermaid
```

Then navigate to the docs folder and run `mdbook-mermaid install`

To view the documentation & have it update as you modify it, run `mdbook serve` and navigate to the URL it provides. Or, to just build it run `mdbook build`.

# Cosmos Roadmap

See [the issues page](https://github.com/AnthonyTornetta/Cosmos/issues) for the list of current features/bugs in development.


## Release 0.0.2a
- [x] Improved state management
  - [x] Client state management
  - [x] Server state management
  - [x] Core able to utilize both states
    - [x] Block resource loaded in PreLoading state
    - [x] Blocks loaded in Loading state
    - [x] Systems loaded in PostLoading state
- [x] Ability to pilot Ship
  - [x] Mouse movements steer ship around ship core
  - [x] Standard movement controls mapped to ship acceleration
  - [x] Interact with ship core to enter piloting mode, press interact button again to exit
  - [x] Create max ship speed
- [x] Ship core
  - [x] Interact with this block to pilot the ship
  - [x] Cannot be mined while other blocks are present on the ship
  - [x] Create block
- [x] Localized planetary gravity
  - [x] All entities near planet's radius are subject to its gravity towards its relative downward vector
    - [x] The gravitational pull should scale inversely exponentially the farther the distance from a certain threshold (most likely the highest chunk) then remain constant past that point
- [x] Ship systems
  - [x] Energy system
    - [x] Energy producer block
    - [x] Energy storage block
    - [x] Energy storage system
    - [x] Energy generation system
  - [x] Thruster system
  - [x] Thruster block
  - [x] Allows the ship to move + rotate
    - [x] Faster movement based on # of thrusters
    - [x] Faster rotation based on # of thrusters
    - [x] More energy consumption per thruster
  - [x] Laser cannon system
    - [x] Laser cannon block
      - [x] Can be placed in lines to create more powerful lasers
    - [x] Click lmb/hold to fire the laser
      - [x] The laser is on a cooldown
  - [x] Ship hull block
- [x] Ability to place more than one block
  - [x] Hotbar
    - [x] Rendering of hotbar
    - [x] Reads items from inventory
  - [x] Inventory
  - [x] Items
    - [x] Item stacks
  - [x] Way of select which item
- [x] Store block damage
- [x] Ability to save ships to disk + load them
  - [x] Implement console commands

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


## Everything that will still have to be done after 0.0.2a
- [ ] Align body with structure
  - [ ] Switches to FPS Camera
  - [ ] Aligns the player to that structure
  - [ ] Left control
  - [ ] De-align, switch back to free camera
    - [ ] Create free camera
  - [ ] Mining beam system
    - [ ] Mining beam block
      - [ ] Can be placed in line to create more powerful miners
    - [ ] Mines the first block hit by the beam after a given time
      - [ ] Inserts the item into the ship's inventory
      - [ ] Hold lmb to continually fire the laser
    - [ ] Structure gets deleted when no more blocks are left
  - [ ] Storage system
    - [ ] An interface into all the storage devices on the ship
  - [ ] A way of selecting which systems to use preventing use of systems that are not meant to be actively used
    - [ ] You can fire a laser cannon, but not actively use the power storage blocks
- [ ] Dropped item entity
- [ ] GUI to interact with inventory
- [ ] Storage block
  - [ ] A block that stores an amount of items
  - [ ] Can be interacted with to view the items
    - [ ] A GUI to view items
- [ ] Camera system
  - [ ] Camera block
  - [ ] Use left/right to switch between ship cameras
    - [ ] Changes where your view is
- [ ] Sounds
  - [ ] Laser cannon fire
  - [ ] Block take damage
  - [ ] Thrusters moving
  - [ ] Space ship idle
  - [ ] Background space ambiance?
- [ ] Planet Generation
  - [ ] New planet types
  - [ ] A bunch of new blocks
- [ ] Galaxy Generation
  - [ ] Stars procedurally generated in spiral-like pattern based on seed 
  - [ ] Planets generate around stars, their biosphere depending on how close they are to the sun
  - [ ] Asteroids
    - [ ] Mineral deposits
- [ ] Shops
  - [ ] Sell blocks/items
  - [ ] Buy blocks/items
  - [ ] Generates randomly
    - [ ] Implement random structure Generation
  - [ ] Prices based on supply + rarity
    - [ ] Keep supply relatively equal between nearby shops
  - [ ] Each shop has its own supply of money that it cannot go below
  - [ ] Implement currency system for player
    - [ ] Money GUI
    - [ ] Pay others
  - [ ] Shop GUI
- [ ] Block
  - [ ] Block resistances
  - [ ] Light emitting blocks
