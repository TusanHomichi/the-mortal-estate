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
hosts a world over the network, by invite, and keeps it through a restart.
The client comes next.

## Hosting a world

```sh
cargo run -p tme-server -- init worlds/home --name "Home" --address HOST:7420
cargo run -p tme-server -- run worlds/home
```

`init` makes the world's folder: its settings (`world.toml`), its key
(`world.key`, which never leaves the folder) and, once it runs, its journal
and snapshots (`world.db`). It prints the invite link to give the people you
want in; `invite` prints it again. Someone new needs the link; someone who's
been before comes back with their own key. `run` keeps the world until Ctrl-C
or a service manager stops it, and picks up where it left off the next time.

## Working here

[AGENTS.md](AGENTS.md) is the first stop. `nix flake check` is the gate, and
plain `cargo test --workspace` runs the same tests.

## Licensing

- **Code and original game content:** [GNU AGPL-3.0](LICENSE). Nothing is
  sold; take it and host your own worlds.
- **Third-party material:** its own license, kept beside the material
