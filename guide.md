# Player Guide

🚧 This is still being made. Please let us know of any suggestions or questions you have. 🚧

**Note:** Cosmos is still very early in development, and all features are still subject to many changes. Expect many bugs.

If you have any questions, please ask in the [Discord server](https://discord.gg/VeuqvnxsZb).

## Steam

You can request access to the playtest in our discord server.

If you want to test features, Creative Mode is a good way to see everything.
Once you create a singleplayer world, to swap to creative open the chat (`Enter`), then type `/gamemode c`. This will give you a creative inventory (open with `T`) to build with.

[Check out the basic gameplay guide to get started.](./guides/gameplay.md)

## Building Locally

### Start the server

```sh
cosmos/cosmos_server$ cargo run --release
```

To run a creative server, add the `--creative` flag. Additional flags can be found with `--help`.

> Note that flags such as `--creative` and `--help` must all be placed after a `--` so cargo recognizes they go to the game, not cargo.
> For example: `cargo run --release -- --creative --peaceful --debug-window`

You can also run commands on the server, see [commands](./guides/commands.md).

#### IMPORTANT SERVER NOTE

Do **NOT** close the server by closing the window that opens or closing the terminal - the world may not be saved.

To gracefully close the server, simply run the `stop` command. You can type this in the server's console, or an operator can type
`/stop` in the game's chat. See [commands](./guides/commands.md) for more information.

### Start the client

```sh
cosmos/cosmos_client$ cargo run --release
```

### !! Important Client Note !!

Singleplayer only works if you have built the server locally. If you haven't, when you try to create a world the game will fail to find the server executable.

[Check out the basic gameplay guide to get started.](./guides/gameplay.md)
