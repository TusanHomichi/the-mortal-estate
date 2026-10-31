# The Mortal Estate

> **Death is another place.**
>
> **A life ends. The world continues.**

The Mortal Estate is an original game about a family line, a land where death
stopped working, and a world that remembers.

You play a lineage, not one hero. The land is an estate whose death was never
settled, so nobody here ages and the dead don't leave: they cross to the other
side of the same land, the land as it was, and keep playing until they find a
way back. Every return leaves a mark. Lives end by choice, passing the line to
an heir or leaving the land for good, and the ancestors stay. Anyone can take
the code and host a persistent world for their family, friends or a crowd,
and it runs until its estate is settled; then that world becomes the past of
the next.

## Status

Building the [first playable slice](docs/DESIGN.md#the-first-playable-slice),
started fresh on 2026-10-30; [docs/DESIGN.md](docs/DESIGN.md) is the spec. The
rules and the world are tested headless: free movement, committed attacks,
the dun, dying into Lately and turning up again, on one small map. A server
hosts a world over the network, by invite, and keeps it through a restart,
and a bot harness measures what it costs. The client plays it in the Lantern
look.

## Hosting a world

```sh
cargo run -p tme-server -- init worlds/home --name "Home" --address HOST:7420
cargo run -p tme-server -- invite worlds/home
cargo run -p tme-server -- run worlds/home
```

`init` makes the world's folder: its settings and invite code (`world.toml`),
its key (`world.key`, which never leaves the folder) and, once it runs, its
journal and snapshots (`world.db`). Both credentials are readable by the host
alone; Windows can't promise that yet, so a world can't be made there until it
can ([#6](https://github.com/TusanHomichi/the-mortal-estate/issues/6)). `invite` prints the link to give the people you want in, and is the one
command that prints its code, so it stays out of a service's logs. The link
names the world's key, and a connection is sealed end to end to that key, so
nothing between a player and the world can read or change what they say. Someone
new needs the link; someone who's been before comes back with their own key. `run`
keeps the world until Ctrl-C or a service manager stops it, and picks up where
it left off the next time, crash or not. One server runs a world at a time,
and if it can't save what's played it stops rather than carry on unsaved.
`world.toml` is read at each start: its settings, and new content the saved
world fits, apply from then on, while what was already played keeps the
content it was played on.

## Playing

```sh
cargo run --release -p tme-client -- 'tme://HOST:7420/…?invite=…' --name Wren
```

The link is the one its host gave you. The first time, the client makes your
key, which is who you are in every world, and keeps it with your other files
(`~/.local/share/the-mortal-estate` on Linux), where only you can read it;
like a world, a key can't be made on Windows until
[#6](https://github.com/TusanHomichi/the-mortal-estate/issues/6). Move with WASD or a stick,
swing with the mouse or J toward the cursor, dodge with space, and use what's
in front of you with E. T turns the tilt-shift blur off and on; `--band`
sets how much of the screen it keeps sharp.

## Measuring a world

```sh
cargo run --release -p tme-bots -- --bots 100 --seconds 30
```

hosts a scratch world on Lattermead and fills it with bots that wander,
swing and dodge. People can hurt each other in that world, so where bots
crowd they die, and walk back from Lately to turn up. It prints how long the server took to compute
each step and how long each intent took to be answered. The numbers belong to
the machine they were taken on, so a release states the ones it was measured
with.

## Working here

[AGENTS.md](AGENTS.md) is the first stop. `nix flake check` is the gate, and
plain `cargo test --workspace` runs the same tests.

## Licensing

- **Code and original game content:** [GNU AGPL-3.0](LICENSE). Nothing is
  sold; take it and host your own worlds.
- **Third-party material:** its own license, kept beside the material
