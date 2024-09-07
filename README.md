# Cosmos

A multiplayer block-based space exploration game, written in rust using the [Bevy](https://www.bevyengine.org/) engine.

If you're interested in playing the game, helping out, pitching ideas, or just hanging out, join our discord server!
[![Join Cosmos's Discord server here!](https://dcbadge.vercel.app/api/server/VeuqvnxsZb)](https://discord.gg/VeuqvnxsZb)

## Screenshots

Cosmos is a game where you can create your dream spaceship that you can pilot through space.
![](./showcase/gunship_top.png)

Build your own ship block by block and cruise through space with friends, or make have everyone build their own ship.
![](./showcase/many_ships.png)

Fly by huge, fully-interactable, planets
![](./showcase/planet_close.png)
![](./showcase/on_planet.png)

Beware! In the depths of space, there's always someone that wants you dead. Watch out for pirates! They spawn in pacts and, once they see you, will hunt you mercilessly.
![](./showcase/shooting_ship.png)

## Compilation

To get started, install your OS dependencies [here](https://bevyengine.org/learn/book/getting-started/setup/#install-os-dependencies). The dependencies section is all you need to do.

This project requires the latest nightly rust to compile. To swap to nightly, run the command

`rustup default nightly`.

To run the client, navigate to the cosmos_client directory and run

`cargo run`

For the server, navigate to the cosmos_server directory and run

`cargo run`

For release builds, append the `--release` flag to the build/run commands.

## Controls

There is no option to modify controls yet, so for now check out `cosmos_client/src/input/inputs.rs` to see a list of all controls currently implemented.

## Documentation

The first time you view the cosmos documentation, make sure you have mdbook **and** mdbook-mermaid installed. If you don't you can install them by running the following commands:

```console
cargo install mdbook
cargo install mdbook-mermaid
```

Every time you want to view the documentation, navigate to the `docs/` directory. To have it update as you modify it, run `mdbook serve` and navigate to the URL it provides, or to just build it run `mdbook build`.

#### System ordering

If you want to view the ordering of the systems, run (on linux) `cargo run --features print-schedule | dot -Tsvg > ./debug.svg` for either the cosmos_client or cosmos_server projects. If the `print-schedule` feature is enabled, these are setup to use bevy_mod_debugdump to create graphs of the `Update` schedule. If you need a different schedule, you'll have to change the `main.rs` file for either project to specify the correct schedule.  Note, you'll need [graphviz](https://graphviz.org/download/) for the dot cmomand to work.

# Cosmos Roadmap

See [the issues page](https://github.com/AnthonyTornetta/Cosmos/issues) for the list of current features/bugs in development.

## Release 0.0.7a (In Progress)
- [ ] Wires
  - [ ] Electrical
- [ ] Galaxy map
  - [ ] Able to view stars
  - [ ] Zoom in to view planets
  - [ ] Set waypoint
- [x] Rotating Planet
  - [x] Planet Skybox
- [ ] Biosphere Improvements
  - [ ] Ice biosphere glaciers
  - [ ] Better Water block
  - [ ] Better Lava block
  - [ ] Structures
    - [ ] Rocks spawning
    - [ ] Undergrowth
    - [ ] 1 More tree type
  - [ ] Groundwork for biomes
    - [ ] Planes
    - [ ] Redwood forest
    - [ ] 1 Additional forest
    - [ ] Ocean
  - [ ] On-planet skybox (maybe)
    - [ ] Sun-side skybox
      - [ ] Perhaps done via a sphere surrounding the planet that always faces the nearest star
    - [ ] Sun-set skybox
    - [ ] Night-side skybox
  - [ ] Volumetric lighting on planets
- [ ] Dropped item entity
  - [ ] When storage is broken, drop items on ground
  - [ ] If not enough inventory room when player is mining something, drop item
- [ ] Fix missing chunks on planets

## Release 0.0.6a
- [x] Place rotated blocks
- [x] Add main menu
  - [x] Customizable settings
  - [x] Specify connection target
- [x] Missiles
  - [x] Auto targetting
  - [x] Explosion logic
- [x] Shields
  - [x] Emit a spherical shield
    - [x] Shield size based on multiblock structure
  - [x] Absorb incoming damage
- [x] Inventory UI improvements
  - [x] Scrollable inventory
- [x] Escape functionality
  - [x] Escape can now be used to close the foremost open UI menus
  - [x] If no such menus are available, it will instead open the pause menu
    - [x] Pause menu
      - [x] Resume
      - [x] Change Settings
      - [x] Disconnect
      - [x] Quit Game
- [x] LOD Fixes
  - [x] Consolidate LOD + Normal chunk rendering into one source of truth
  - [x] LOD performance improvements via combining meshes + culling sides
- [x] Block data
  - [x] Storable fluids
- [x] Item data
- [x] Money
- [x] In-flight UI
  - [x] Weapons selection hotbar
  - [x] Speed display
  - [x] Energy display
  - [x] A way of selecting which systems to use preventing use of systems that are not meant to be actively used
- [x] Wires
  - [x] Logic
    - [x] Logic blocks
    - [x] Wiring system
    - [x] Logic receivers
- [x] Planet generation & LOD overhaul
  - [x] Performance improvements
  - [x] GPU-based generation
- [x] Remove bevy system ambiguities
- [x] Support NxN texture loading (where n is power of 2)
- [x] Update to bevy 0.14

## Release 0.0.5a
- [x] Add a gravity well block
  - [x] Remove snapping to structures on collision
- [x] Camera system
  - [x] Camera block
  - [x] Use left/right to switch between ship cameras
    - [x] Changes where your view is
- [x] Shops
  - [x] Sell blocks/items
  - [x] Buy blocks/items
  - [x] Generates randomly
    - [x] Implement random structure Generation
  - [x] Implement currency system for player
    - [x] Money GUI
  - [x] Shop GUI
- [x] Mining beam system
  - [x] Mining beam block
    - [x] Can be placed in line to create more powerful miners
  - [x] Mines the first block hit by the beam after a given time
    - [x] Inserts the item into the ship's inventory
    - [x] Hold lmb to continually fire the laser
- [x] Pirates
  - [x] Create a number of pirate ships
    - [x] Each ship has a difficulty
  - [x] Difficulty of spawns scales off player's money & ship's number of blocks
  - [x] Basic AI
    - [x] Spawn within 2 sectors of player
    - [x] Spawn in fleets based on player's difficulty
    - [x] If within ~1 sector of player, fly towards player
    - [x] Shoot towards player, trying to predict their position based off distance & velocity
- [x] Shop
  - [x] Buy from shop
  - [x] Sell to shop
  - [x] Players have money
  - [x] For now shops have unlimited stock & funds
  - [x] Shop block
- [x] Animated textures
- [x] Block data system
  - [x] Support for blocks that store their own data
  - [x] Storage block
    - [x] A block that stores an amount of items
    - [x] Can be interacted with to view the items
      - [x] A GUI to view items
- [x] Sounds
  - [x] Laser cannon fire
  - [x] Block take damage
  - [x] Thrusters moving
  - [x] Space ship idle
  - [x] Background space music
  - [x] Block place/break
- [x] Multiblock machines
  - [x] Revamp power generation to use reactor multiblock structure
- [x] Colored laser
  - [x] Colored glass placed in front of laser
- [x] Inventory GUI
  - [x] Able to open inventory
  - [x] Abstract the 3d block GUI camera
  - [x] Fix the 3d block GUI camera to not render anything except GUI blocks
- [x] Fix seeing through cracks in blocks
- [x] Lods
  - [x] Reduced detail rendering of far away chunks to see the entire planet
- [x] Update to bevy 0.11
  - [x] Update physics
  - [x] Update bevy
- [x] Update to bevy 0.12
  - [x] Update physics
  - [x] Update bevy
- [x] Update to bevy 0.13
  - [x] Update physics
  - [x] Update bevy
- [x] Update asteroid generation to make it decent
  - [x] Add ores to mine on them
- [x] GUI to interact with inventory
  - [x] Easier way of adding 3d blocks to GUI
  - [x] Move items around in inventory via mouse
- [x] Structure build mode
  - [x] Interact with build block to enter build mode
  - [x] Build mode
    - [x] Camera becomes a noclip free cam and goes outside of player's body.
    - [x] Player no longer piloting ship, & is able to create + destroy blocks on the ship but ONLY the ship
    - [x] Symmetry modes are added that will mirror blocks on user-defined axis

## Release 0.0.4a

- [x] Galaxy Generation
  - [x] Stars procedurally generated in spiral-like pattern based on seed
    - [x] Create star
      - [x] Light emits from star
      - [x] Load star within system
  - [x] Planets generate around stars
    - [x] Biospheres depend on how close they are to the sun
    - [x] Dynamic biospheres based off temperature
    - [x] Only generate if planet is close enough to player
    - [x] Cube planets
      - [x] Planets will now be cubes instead of flat planes, and will be about the size of the sector
      - [x] Redo saving/loading of planets
      - [x] Dynamically generate chunks & unload them based on players' positions close to planet
      - [x] Make generation work on all faces of planet
      - [x] Block orientation for every block
  - [x] Biospheres
    - [x] Speed up terrain generation
    - [x] Make molten biosphere
    - [x] Enhance grass biosphere
    - [x] Create icy biosphere
  - [x] Asteroids
    - [x] For now just floating rocks in space
  - [x] Save generated universe
    - [x] Save planet locations
  - [x] Fix broken ship functionality
    - [x] Make entities no longer pass through loading structures.
- [x] Align body with structure
  - [x] Switches to FPS Camera
  - [x] Aligns the player to that structure
    - [x] Fix child locations being not updated based on transform relative to parent
  - [x] Add button to align to structure facing
  - [x] De-align, switch back to free camera
    - [x] Create free camera

## Release 0.0.3a

- [x] Infinite universe
  - [x] (**Client**) Player can travel any distance from 0,0,0 with no noticable issues, and everything moves relaitve to player
  - [x] (**Server**)
    - [x] Objects move relative to player world they are a part of
    - [x] Player world moves relative to its player
    - [x] Players can be a part of the same world if they are close enough
    - [x] Requires rewrite of bevy_rapier to support multiple physics worlds
      - [ ] Merge PR https://github.com/dimforge/bevy_rapier/pull/328
- [x] Update to bevy 0.10.0
- [x] Performant ships
  - [x] Ability to have 50 small-sized ships loaded with minimal performance impact
- [x] Dynamic object loading
  - [x] (**Client**)
    - [x] Unload objects that are too far away
    - [x] Request entities that server sends if they don't have the entity for them
  - [x] (**Server**)
    - [x] Unload objects that are too far from any player
    - [x] Load objects that are close to a player & send that info to client
    - [x] Only send information about objects to clients that are close enough to have them loaded
- [x] Display coordinates in top left (sector, local)
- [x] Revamp rendering to allow for more than cubes
- [x] Display hotbar items
  - [x] 3d models for blocks

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

## Release 0.0.8a
- [ ] More pirate ships
- [ ] More asteroid types
- [ ] Fuel for reactor
- [ ] Performance fixes
- [ ] Fix server instability
- [ ] Reduce amount of packets server sends per second
- [ ] Rebalance shop prices
- [ ] Add more randomly generated structures (e.g. stations)

## Everything that will still have to be done after 0.0.7a
- [ ] Shops
  - [ ] Peace zone?
  - [ ] Prices based on supply + rarity
    - [ ] Keep supply relatively equal between nearby shops
  - [ ] Each shop has its own supply of money that it cannot go below

## NPCs

### Factions

NPC controlled OR player controlled
NPC controlled factions store reputation of other factions + players
Factions have different attributes
  - One may steal your ship blueprints
    - To balance this:
      - scrap is collected when you mine melting down ships instead of the actual blocks
        - Different scrap types
      - Substitute expensive blocks for less expensive ones if needed
      - If ship is below 50% success rate, don't produce it again
  - Dynamic faction expansion
Bounty board
  - Take down X ship
  - Selling the ship
- Selling ship designs
- Buying ship designs

