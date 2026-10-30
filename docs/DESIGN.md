# The Mortal Estate: design

The design the game is built toward. It's a draft: the
[decisions table](#decisions) holds the owner's calls, and the rest are
proposals that hold until a playtest or the owner says otherwise. A change to
the design changes this file in the same pull request.

## The game

You play a family line, not one hero. Each life in the line grows, wears and
ends, and the line goes on: heirs inherit what their ancestors carried, learned
and owed. In this land death doesn't end anyone. The dead cross to the other
side of the same land, the land as it was, and keep playing there until they
find their way back. Every return leaves a mark, so a life slowly drifts
toward the dead side, and sooner or later the line passes it on. The ancestors
stay. The world remembers who did what, and the past is a place you can walk.

Worlds are persistent, and anyone can take the code and host one: for a
family, a group of friends, or a crowd, as big as the game supports. A world
is a campaign: it runs until its estate is settled, then it ends and becomes
the past of the next world, where the same families carry on.

## The land

The land is an estate whose death was never settled. Whoever held it died and
nothing was ever divided, paid or closed, so death itself stopped working here:
nobody ages, and the dead don't leave. The player families are its heirs, each
with a claim, a share of its debts and a stake in what happens to it. Settling
the estate (what was owed, what was promised, who gets what, what really
happened) is a world's long arc, and when it's done, that world is over.

The tone is melancholy and warm: a land full of unfinished business, people
who care about each other across death, grim moments and real danger, and room
for the odd and the funny. It's never grimdark.

## Pillars

Every feature serves at least one of these. One that serves none doesn't ship.

1. **You are a lineage.** The player's continuity is the family line. A life
   is a chapter: it has a beginning, a shape and an end the player chooses.
2. **Death is a place.** Dying moves you to the dead side, where you keep
   playing. It's never a loading screen or a lost evening.
3. **The world remembers.** People, places and objects keep a record of what
   happened to them. Former characters remain people in the world.
4. **Play together.** The living and the dead need each other, and the rules
   reward helping. Nothing punishes the player who shares a kill or a find.
5. **Deliberate danger.** Every swing and spell commits you, terrain matters,
   and you see death coming. Skill is choosing well, not clicking fast.

## Decisions

| Date | Call | By |
| --- | --- | --- |
| 2026-10-30 | Start fresh: an original design, built new, keeping nothing from the earlier build | owner |
| 2026-10-30 | Lineage and death stay the heart: a family line, death as a place, a world that remembers | owner |
| 2026-10-30 | Anyone can take the code and host their own worlds, as big as they like up to the size the game supports; one server runs one persistent world | owner |
| 2026-10-30 | Nothing is sold: the code and all original content are open under the AGPL, so hosting is how the game spreads | owner |
| 2026-10-30 | The client and the server are both handed out, and a world browser in the client lists the worlds their hosts make public, so joining one takes a click | owner |
| 2026-10-30 | Each world is set up its own way, and joining one fetches whatever it runs (its release, content and settings) and later patches only what changed | owner |
| 2026-10-30 | Our world list is a free Cloudflare Worker that holds only addresses and keys; it ships with the first public release, and invite links come first | owner |
| 2026-10-30 | Our list shows only worlds running a supported release of our server; anyone can fork and invite players, but forks list elsewhere | owner |
| 2026-10-30 | Setting up and changing a world is built for a host working with an AI assistant, and how easy it is gets measured | owner |
| 2026-10-30 | The look is stylized low-poly 3D: open asset kits as the base, the rest generated to the same style rules | owner |
| 2026-10-30 | The style is Lantern: a diorama at dusk with long shadows, glowing windows and a tilt-shift blur that keeps most of the view sharp; the dead side is lit teal from below | owner |
| 2026-10-30 | Free movement and committed actions: no tiles and no action timers; attacks and spells wind up, land and recover | owner |
| 2026-10-30 | Characters grow by doing: skills by use, no classes, heirs inherit a share | owner |
| 2026-10-30 | The land is an estate whose death was never settled; the families are its heirs, and settling it is a world's long arc. Melancholy and warm: grim moments, not grimdark | owner |
| 2026-10-30 | A world ends once its estate is settled, and becomes the dead side of the next world, where the families carry on | owner |
| 2026-10-30 | The dead side is called Lately: the dead have "gone to Lately" and are the Lately | owner |
| 2026-10-30 | The rest of the names riff on being late: coming back is turning up, marks are tardies, succession is passing the kettle, departure is going on time | owner |
| 2026-10-30 | Settling an estate means paying its debts, rolled fresh for each world and owed to the living, the Lately and the land; once they balance, the will is read and the world ends | owner |
| 2026-10-30 | The title stays The Mortal Estate | owner |
| 2026-10-30 | The client is Bevy, so the whole game is Rust; the simulation stays plain Rust, so the engine's changes only touch the client | owner |
| 2026-10-30 | No list of banned names and no check for one: the game takes nothing from another game, so the originality rule needs only review | owner |

## A life

### Growing

There are no classes. A life gets better at what it does: skills rise with use
and with teaching, and fall back slowly when neglected. What a life is good at
is its history, readable on its sheet as the things it did.

Heirs inherit a share of their ancestor's skill (bounded, so a new life still
has to earn its own), which means a family drifts into a tradition over
generations: a line of menders, of delvers, of people who are very good with an
axe. A lineage can change course; it just costs a generation.

### Dying and the dead side

Every death crosses over. There is no waiting by your body and no separate
afterlife map: the dead side is the same land, as it was. The people of the
land call it **Lately**, after the late: the dead have gone to Lately, and
they're the Lately.

- **What you see.** The dead side is the land in an earlier state: the burned
  hall standing whole, the bridge before it fell, the people who lived there.
  Things the living have destroyed or lost stay on the dead side as they were.
  The living are faint there, and the dead are faint to the living.
- **What you carried** stays where you fell, in the living world. Anyone can
  find it. Returning lets you go get it; friends can recover it for you.
  Nothing lies around forever: what's left unclaimed long enough gathers
  into a pile that sinks into Lately, where the dead can go through it for a
  while before it fades. How long, and what goes in a pile, is for playtests.
- **What you can do.** The dead walk, talk, fight what lives on the dead side,
  learn what only the past knows (where something was buried, a way in that no
  longer exists), and help the living with it.
- **Coming back**, which the land calls *turning up*, is earned, never
  automatic: reach a threshold where the two sides touch and pay its price, or
  be called back by the living through a rite that needs something of yours
  (your remains, an heirloom, your name spoken by kin).
- **Marks**, which the land calls *tardies*. Every return leaves one: a
  lasting trait from the dead side. Some help (hearing the past, seeing the
  dead), some cost (a weaker body, the distrust of the living). Marks replace a
  flat penalty: death changes a life instead of just shrinking it. A life heavy
  with marks returns less easily, and eventually not at all; it stays playable
  on the dead side for as long as the player wants.

### Ending a life

A life ends one of two ways, and each is its own kind of event in the code,
never a flag on a death.

- **Succession**, which the land calls *passing the kettle*, after the family
  kettle the heir takes. A deliberate rite at a place of the lineage. The
  ancestor joins the dead side as a resident and keeps their record; the heir
  is a new person (new name, new face) who inherits the heirlooms, a share of
  the skill and the family's standing. It can't happen by accident or by
  timeout.
- **Departure**, which the land calls *going on time*: everyone in the land is
  late, and leaving is the only way to die on time. The one way to truly
  leave: walk out of the land and accept ordinary age and final death. It's
  one-way, confirmed deliberately, and closes that life's story for good. It
  counts as a life completed.

A life too marked to return isn't over: it stays on the dead side, playable,
until the player holds the succession rite for it there.

Nobody ages in the land. That's the premise: mortality is broken here, and the
only way to have an ordinary death is to leave.

## The lineage

- **Heirlooms** are objects that move between lives with their history intact:
  who carried them, where, what they did. An heirloom is one object that moves,
  never a copy.
- **Ancestors** are people on the dead side. Players, theirs and other
  families', can meet them, ask them things and be helped or hindered by them.
  What an ancestor says and does comes from their recorded life.
- **Standing.** What the world remembers about the family: debts, favours,
  grudges, deeds. Specific memories, never a reputation meter.
- **Households.** Lineages in a world can bind together, share a hearth and
  hold rites for each other's dead.

## The world

- **One persistent world per server.** It keeps every change and
  survives restarts and updates without losing or duplicating anything.
- **A world is a campaign.** It lasts weeks to months, until its estate is
  [settled](#settling-the-estate). Then it ends: its land, its people and its
  dead become the dead side of a new world, and the families carry on into it
  with their heirlooms and ancestors. A host can start a world fresh, or from a
  settled one.
- **The land changes.** Players build, burn, clear and claim. The living world
  is the present; what the living destroy or lose stays on the dead side as it
  was, so a world grows its own past as it's played, on top of the past it
  started from.
- **World memory is specific.** A person remembers that you pulled them out of
  the river, not that your standing is 42. Memory is keyed by who and what
  happened, and only the owning code reads or writes it.
- **Places have a rhythm.** Hunting grounds that refill at the pace of a
  circuit, lairs where the danger is the terrain itself, open country that
  isn't crowded.

## Settling the estate

Each world rolls its own estate when it starts: who held it, what they left,
and what it owes. The rolling draws on building blocks in `content/` (kinds of
people, places and debts), seeded per world, so no world's answers ever sit
in this repository.

- **The debts are the arc.** The estate owes in three directions: to the
  living (a loan never repaid, a field never handed back, a promise to a
  household), to the Lately (a burial never done, a name never spoken, a wrong
  never owned up to) and to the land itself (a choked spring, a burned wood, a
  boundary stone moved). Some can be paid on one side only; most need both.
- **The ledger starts mostly blank.** Debts surface as the world is played:
  the Lately remember them, old papers turn up, the land shows them. Finding
  one is part of paying it.
- **Paying is a deed.** A debt is paid in the world, by anyone, alone or
  together, and the world remembers who paid it. A debt can also be forgiven by
  whoever it's owed to, or passed down a lineage; it can never be bought off.
- **The reading.** When the ledger balances, the families gather at the
  estate's hall and the will is read. It divides the estate by what each family
  did, and the world ends: it becomes the Lately of the next world, where the
  families carry on.

A world holds debts in proportion to its people, a few dozen for a small
group, sized so its families settle it in weeks to months.

## Playing together

- Credit is shared: everyone who fought gets their part, and finding something
  doesn't bind it to whoever touched it first.
- The two sides are complementary: the dead see the past, the living hold the
  present, and the best play uses both at once.
- Harm between players is a world setting the host chooses, off by default.
- Social tools (chat, tells, a log you can scroll back) are modern and easy.
  What you hear in the world, like speech in sight and shouts, stays part of it.

## Finding a world

Anyone can download the client, and the server too if they want to host.

- **Private by default.** A host shares an invite link; opening it starts the
  client and joins the world.
- **Public if the host says so.** A host can list a world, and it shows up in
  the client's world browser with what a player needs to pick one: its name and
  the host's description, who's playing against its cap, how far along its
  estate is, its house rules (harm between players, language), its version, and
  the ping from where you sit. Joining is one button.
- **Each world is its own setup.** A host picks the release the world runs, its
  content (ours, someone else's or their own) and its settings. The browser
  shows all of it before you join, with how much there is to download and how
  much of it you already have.
- **Joining fetches what's missing.** Your client downloads the world's content,
  and the release it runs if you don't have it, and on later visits patches
  only what changed. You walk in once the place you'll stand is loaded; the
  rest streams in behind you, nearest first.
- **No signup.** Your client keeps a key, and that key is who you are. A world
  makes your account the first time you join, so there's no password per world.
  The key moves with you to a new machine, and a host can re-link a lost one.
- **A list is a phone book.** It holds each listed world's address and key,
  nothing else; the browser asks each world directly for its name, players,
  rules, estate and ping. A list only shows a world it can reach, and a world
  drops off when it stops telling the list it's up.
- **Ours lists only supported releases.** A world must run a release of our
  server that we still support. A changed server can claim to be one, so the
  check is on what it reports, and a report about one takes it off.
- **Anyone can run a list.** A fork can't join ours, but it can invite players
  and run a list of its own. The client ships pointing at ours and takes others
  in its settings.
- **Keeping it decent.** Players can report a listing; a report hides it until
  whoever runs the list looks. What happens inside a world is its host's
  business.

## Making a world

Setting up and changing a world is made for a host with an AI assistant doing
the typing, and stays easy for a host without one.

- **A world is one folder:** a settings file and whatever content the world
  adds, all plain text, one entry per file. Starting a new world asks a few
  questions (name, size, harm between players, which content) and writes a
  world that runs.
- **Every format has a schema** that the tools publish, and each new world
  folder comes with a guide written for assistants: the formats, the commands,
  the checks and what each setting does.
- **Checks explain themselves.** Checking a world names the file, the line,
  what's wrong and what would fix it, in words a person or an assistant can act
  on, with a machine-readable form beside it.
- **One set of commands, two ways in.** The host's tools are a command line
  with machine-readable output, and the same commands are offered to an
  assistant as tools, with the host's authority on the host's machine.
- **Changes are rehearsed and undoable.** A change to a live world can run
  first on a copy replayed from the world's own journal, so the host sees what
  it does before it lands. Every change to a world's setup is recorded and can
  be undone.
- **Easy is measured.** Tests set up and change a world from nothing using only
  the documented commands, so the guide and the tools can't drift apart, and a
  step that gets harder shows up as a failing test.

## The core loop

| Scale | Loop |
| --- | --- |
| Moment | Move freely, fight, use and talk; every swing and spell commits you |
| Outing | Leave the settlement, push into danger, bring back things and knowledge, or die |
| Death | Cross over, walk the past, help or be helped, earn a way back, gain a mark |
| Life | Grow by doing, wear by dying, choose the ending: succession or departure |
| Lineage | Heirlooms gather history, ancestors gather on the dead side, a tradition forms |
| World | The land changes under its families until the estate is settled; then the whole world becomes the next one's past |

## Moving and fighting

- You move freely, by stick, keys or click to move; there are no tiles. A
  place is a full address (world, area, level and position), never just a name.
- Every control scheme plays the same game: a controller, keyboard and mouse,
  or touch on the web build.
- Actions commit. An attack, a spell or a use winds up, lands and recovers, and
  once you start one you're in it until it's done. The only way out of a
  wind-up is a dodge, which commits you too. Leaving the world isn't a way
  out either: your body sees the action through, then goes.
- Creatures commit the same way and show their wind-ups, so you see the blow
  coming and can be somewhere else. Where you stand is most of the skill:
  range, cover, footing, and who's beside you.
- Hazards build. Water, cold, fire and falls get worse over several moments,
  with clear warnings, so a death is always seen coming.
- There are no cooldown bars: an action's cost is the time it holds you.
- Input while you're committed is refused, not queued, so nothing fires later
  that you didn't mean.
- Every number (wind-ups, damage, speeds, skill rates) is designed and tuned
  here, by playtest.

## The look

Stylized low-poly 3D with rigged, animated characters. Open asset kits (CC0 or
similarly free) are the base and set the proportions. Whatever the kits don't
have is generated: each such model is made by a script in this repository, so
it can be regenerated and changed, and every model, kit or generated, follows
one style guide (palette, proportions, edges, materials). Adding a creature or
a building is assembly and generation, not hand drawing.

The style is **Lantern**: the land as a tabletop diorama at dusk. A low sun
throws long shadows, windows glow, colours are warm and a little muted. A
tilt-shift blur softens the top and bottom edges of the view; the sharp band
follows your character and covers most of the screen, so the blur sets the
mood without hiding anything you need to play. The band's width is a tuned
number, and a setting.

The dead side is the same scene with its own lighting, materials and sound: in
Lantern, cold teal light from below and paler colour. The cost of a second
world is mostly a shader, not a second set of art.

The camera looks down on the land at an angle, the view follows your
character, and what you can't see is hidden from you, not just darkened.

## Not borrowed

The Mortal Estate doesn't recreate any existing game. Where it uses a genre
convention (skills, telegraphed attacks, a camera above the land), the design comes from the pillars
above, not from somewhere else. Nothing here copies another game's names,
text, maps, numbers or art. Third-party assets come only from openly licensed
kits, with their licenses kept beside them; generated models come from scripts
here, never from another game's assets.

## How it's built

### Shape

```text
content/ (data) ──► content crate ──► server ◄── intents ── client (Bevy)
                     typed catalogue    authoritative  ── observations ──►  renders 3D
                                        world, timing,
                                        persistence
```

- **Rust** for all of it: rules, simulation, server, client, content
  compilation, tools. One Cargo workspace, one build.
- **The server is the only authority.** The client renders what it's sent and
  sends intents; it previews but never decides.
- **The world lives in memory.** One simulation thread owns it and steps it at
  a fixed rate (about 20 steps a second until a measurement says otherwise):
  movement, actions under way, creatures, effects. Network tasks hand it
  intents through a channel. Nothing locks the world per request. Positions
  are whole millimetres and durations whole steps, so a step works out the
  same on every machine.
- **Movement is predicted, never decided, by the client.** Your client moves
  you at once by running the simulation's own movement code, and the server
  confirms or corrects it; everyone else is drawn a moment in the past,
  smoothed between server updates.
- **Persistence is a journal and snapshots.** Accepted actions append to a
  journal in SQLite, written by its own task off the simulation's path;
  snapshots are taken regularly. Restoring is a snapshot plus the journal since,
  which works because the simulation is deterministic: a seeded random source
  and a step count, never wall time.
- **Observations are per observer and incremental.** The world is indexed in
  coarse spatial buckets; each client gets what changed within its view, not a
  fresh copy of everything nearby.
- **One wire contract.** Messages are Rust types in a crate that the server and
  the client both depend on, so there's one definition and nothing to generate
  or copy. WebSocket transport (it serves the desktop and web
  builds alike) and a compact binary encoding, until a measurement says
  otherwise.
- **The client** is Bevy, pinned to one release and upgraded when we choose,
  not on every release. The simulation is plain Rust with no Bevy in it, so an
  engine upgrade only touches the client. Models and scenes come in as glTF,
  from the kits and from the scripts that generate the rest, with Blender as
  the editor where one is needed. It builds for Linux (the Steam Deck
  included), Windows and macOS, and for the web for a quick join.
- **Content is data.** Places, creatures, items and rites are files under
  `content/`, compiled into typed catalogues with typed ids. A reference to
  something that doesn't exist fails the build. Each fact has one home. Each
  entry is its own file, so parallel branches never fight over one long list.
- **Hosting** is one binary and one data directory, on any ordinary Linux,
  macOS or Windows machine; no particular distribution or package manager is
  required. The host invites players or lists the world; accounts belong to
  the world. How many players a world supports is measured by the bot harness
  and stated with each release, never promised ahead of a measurement.
- **Releases** have version numbers, and each says how long it's supported;
  that window is what our list accepts. Players see what changed in a
  changelog, and a button in the game sends the host a problem report with the
  state around it.
- **The world list** is a small Rust library in the same workspace with two
  thin wrappers: an ordinary binary anyone can run, and the Cloudflare Worker
  that runs ours on the free tier, away from any machine of ours. It stores
  addresses and keys; it never sees a player.

### Joining and patching

- **Everything is stored by its hash.** Releases, content and assets are cut
  into chunks and kept by the hash of what's in them, in one store on your
  machine that every world shares. Two worlds using the same models download
  them once, and an update downloads only the chunks that changed. Chunk
  boundaries follow the content, so an edit in the middle of a big file changes
  the chunks around it, not everything after it.
- **A world's manifest names exactly what it runs:** its release, every chunk
  of its content, and its settings. The world signs it and sends it over its
  own connection, so nothing in between can swap it.
- **Any source will do.** Every chunk is checked against its hash, so the
  client can fetch from the world's server, a mirror, ours, or another player
  on the same network, and trusts none of them. A busy world doesn't have to
  serve every download itself.
- **Content is never code.** What a host ships is data in formats that can't
  run anything: models, textures, sounds, text and typed catalogues. The rules
  run on the server, so a host changes them without anyone downloading code.
- **Code comes only from signed releases.** The client keeps several releases
  side by side and runs whichever one a world needs, but only releases signed
  by us, or by a fork's key the player chose to trust. A world reached by
  invite or another list says plainly before you join when it runs a fork.
- **Loading follows the player.** The manifest groups chunks by place, so the
  client fetches where you'll stand first and the rest by distance. Joining
  takes seconds, not a full download.
- **The store looks after itself.** It keeps what the worlds you play need and
  drops what nothing has used in a while; its size cap is a setting.
- **The web build** uses the same chunks and manifests, held in the browser's
  own storage.

### AI

AI may voice, prepare and play; it never decides. The player API is the same
observation and intent contract the client uses, filtered by role, so an AI
client is just another player and holds no extra power. An optional model may
give an ancestor a voice drawn from their recorded life; it can't change the
world, and everything plays the same without it. Helping a host
[make a world](#making-a-world) is preparing: the assistant works through the
host's own tools, and the host decides.

### Rules for the code

- One owner per fact and per kind of change: one place writes life state, one
  writes an item's location, one advances time.
- An item is in exactly one place. Moving it is one transaction; nothing is
  ever copied to move it.
- Succession and departure are their own events, distinct in type from death,
  so no failure can turn one into another.
- No parallel implementations: a replacement deletes what it replaces.
- Files stay under about 1,000 lines; split by what they own.
- The schema resets freely until a world has players. After that, migrations
  move forward only.

### How it's checked

- Rules and simulation are tested headless in Rust, step by step: a scenario
  runs a world forward and asserts what happened. No test waits on real time.
- A bot harness drives headless clients against a real server from the first
  playable build, and records CPU per step and input latency, so scale is
  measured, not hoped.
- The client is tested headless too, stepped like the simulation: input, and
  the look against reference frames.
- A test world can start a character at any stage of a life or a lineage, so a
  playtest begins where the question is.
- `nix flake check` is the gate, the same locally and in CI; plain `cargo` runs
  the same checks.

## The first playable slice

The skeleton aims at one small place, both sides of it:

1. A server any of the players can host, joined by invite from two clients.
2. A hamlet and a ruin on one small map, in kit assets and the Lantern look,
   with the camera, free movement and committed attacks.
3. One kind of creature that can kill you.
4. Death: you cross to the dead side of the same map (the ruin whole, the
   hamlet younger), see the living faintly, and they see you.
5. A threshold that brings you back, with a mark.
6. The world, both sides, survives a server restart.

Succession, heirlooms and ancestors are the second slice.

## Open questions

None right now. A question waiting on the owner goes here until it's decided,
then moves to the decisions table.
