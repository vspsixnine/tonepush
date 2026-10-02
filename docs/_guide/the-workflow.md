---
title: The Workflow
description: How the pedal, your library and your setlists fit together, and the loop that takes a tone from an idea to a gig.
nav_order: 4
---

The pedal, your library and your setlists are three different things, and knowing which is which is most of knowing how to use TonePush.

**The pedal is where you make a tone**, because it is where you hear it. Everything you change goes to the pedal's scratch buffer, so it is audible the moment you change it, and none of it is permanent until you save.

**Your library is where tones live afterwards.** It is on your computer, not on the pedal, so it outlives any slot on any device. Sell the pedal, buy another, and your tones are still there.

**A setlist is a whole pedal kept as one thing.** Every slot the connected
device provides (126 on an HX Stomp or 60 on a StompStation PRO) in order, as they
were on the night they worked.

## The loop

### 1. Make a tone on the pedal

Swap blocks from the browser, turn knobs, drag to reorder the chain, run something in parallel. The dot beside the preset name goes amber the moment anything differs from what is stored, and `Ctrl+S` commits it.

You do not have to save to keep a tone, though. That is what the next step is for.

### 2. Keep the ones worth keeping

Right-click a preset in the list and choose **Keep in library** to copy it into your library. What goes in is the device's own document, byte for byte, so the snapshots and the routing come with it. Nothing is rebuilt from what the editor happens to show, which means nothing is quietly dropped. A check beside a preset means your library holds it unchanged; an orange compare mark means your library holds a different version under that name, and the same menu offers **Update in library**.

Tones in your library are ordinary files in an ordinary folder. You can back them up, sync them, or read them with something else.

Every tone says which pedal it is for, in the **Pedal** column of the library and of the Cloud, beside a setlist's name, and in a tone's details: the family, then the model (**HX Stomp**, **HX Effects**, **HX Helix LT**) or, for a StompStation PRO, the firmware its chain needs (**PRO 2.x** when the preset has a chain layout, **PRO 1.5** when it has none). TonePush records the pedal a tone was kept from, and its firmware, when you keep it; for a tone kept before it did, it reads the tone itself, which tells a Stomp from an Effects or a Helix but not a Stomp from a Stomp XL. A tone the connected pedal cannot play has a dashed marker and a ban mark where the pedal mark would be, with the reason on hover: another family, another model (an HX Stomp XL reads HX Stomp tones, not the other way round), a tone made on a newer firmware release (patch releases count as the same release), or a PRO tone with a chain layout on a PRO still on 1.5. With the library scoped to the connected pedal, those tones are left out; **All pedals** shows them.

### Hear a tone with one click

A click on a tone in your library plays it on the pedal, in the place of the preset that is loaded. Nothing is written to the pedal's memory: the loaded preset is set aside with its unsaved changes and its undo history, and a bar along the top of the library says what plays and what waits. Click another tone, or press ↓ and ↑ to step through the list, and each one plays against that same preset; Space plays the chosen tone, or puts back the one playing. Ctrl-click and Shift-click choose tones without playing them.

- **Put back** (Esc) writes the preset back exactly as it was, changes and all.
- **Keep in 01B** (Enter, named after the loaded slot) makes what plays that slot's edit: Save then writes it over the preset, and Undo still brings the preset back. Its menu also offers **In another slot…** (Ctrl Enter): choose a slot on the left, and it is written once the preset set aside is back.

While a tone plays, Save is off and Ctrl S points at the bar instead; the deck, the chain and the preset's row in the sidebar say what is auditioned. Switching between the library's tabs leaves it playing. Clicking another preset puts it back first, then asks about its changes as any switch does, and clicking the loaded preset's own row puts it back. A tone the pedal cannot play says why where the bar would be, and offers to show only what the pedal plays. A StompStation PRO auditions the same way, and so does a tone from TonePush.

### 3. Build a setlist

Get the pedal holding the presets you want, in the order you want them, then click the computer icon at the top of the preset list (**Keep every preset on the pedal, in order, as a setlist**), or **Keep the whole pedal as a setlist** on the library's Setlists tab. That records every slot and what is in it; name it, and it opens on that tab.

Give it the name of the gig, the venue, the date. You will want them later; click any of them on the setlist's page to change it.

### 4. Play it back

The library's Setlists tab shows each setlist against what is on the pedal now: its card says whether it matches the pedal or how many slots differ, and its page lays the slots out bank by bank, marking the ones that hold another preset and naming the ones it would empty. The switch beside the count narrows the banks to the slots that differ.

**Put on HX Stomp…** (named after your pedal) writes the whole thing back. It asks first, and says what the write changes: the presets it replaces, the slots it empties and fills, and how many already match. It writes every slot, in order, so tick **Keep the pedal as it is now as a setlist first** if you want what is on it now as a setlist too; nothing is written until that copy is in your library. On an HX pedal the pedal as it was is also kept in its backups (see below).

If you only need one preset out of one, double-click its slot, or right-click it and choose **Send this preset to its slot**.

## Changing a setlist

Put it back on the pedal, edit there, keep the changed tones to your library, and capture a new setlist.

A setlist is never edited in place. That looks like a limitation and is not: a setlist is a record of a rig that worked on a particular night, and a record you can edit is not a record. Renaming a tone next month should not reach backwards and change what you played in March.

This is also why deleting a tone from your library never breaks a setlist. If a setlist still plays it, the tone is kept for that setlist even after it leaves your library.

## Backups

Every time an HX pedal connects, TonePush reads all of it, presets, impulse responses and settings, and keeps that copy current after every save. Before it reads the pedal again, it sets the copy it had aside: when the pedal connects, before a setlist or presets are written to it, and before a restore. The last ten are kept.

**Pedal, Backups** lists them, newest first, with when and why each was taken, beside the backups you saved with **Back up to a file…** (TonePush remembers where the last twenty went, in its settings file). Choose one to compare it with the pedal now, preset by preset ("Ambient Swell edited since", "Sparkle Verb was Night Verb"). **Restore the whole pedal…** writes it back, presets, impulse responses and settings, after saying what it writes; the pedal as it was stays in the backups. **Show in folder** opens the copy itself. The StompStation PRO keeps its own verified backups; see the [StompStation PRO guide](/stompstation-pro/).

## Publishing a Song and Tone

On [TonePush](https://tonepush.rocks), a **Song** is the musical idea: either a catalog song by an Artist or an original. A **Tone** is one playable, device-native preset belonging to that Song. Publishing from the library creates the Song first and then attaches the publishable device artifact (`.hlx` for Line 6 or `.vxpreset` for StompStation PRO) as its first Tone. If adding the Tone fails, the editor says that the empty Song remains instead of pretending the two requests were one transaction. The Tone is listed for the pedal it was kept from and the firmware it was made on, whatever pedal is connected when you publish it.

**Export for the web** writes the same information without publishing it: the
Tone's `.hlx` or `.vxpreset`, plus a `.json` manifest with separate `song` and
`tone` objects. Song facts include its title, kind, Artist, description and
tags; Tone facts include the preset name, part, guitar, tuning and
device-specific description.

Song search results are musical ideas, not files that can be installed. Open a Song and choose one of its Tones for your device before downloading or installing it. An externally indexed Tone opens its original source; a native Tone downloads its hosted artifact.

## Where everything is

| | Where | What it is |
|---|---|---|
| Tones | `~/.local/share/tonepush/library` | One file per tone, the pedal's own document |
| Setlists | `library/setlists` | Small JSON files naming the tones they play |
| Device backups | `~/.local/share/tonepush/backups` | Whole-pedal HX backups (the copy kept current, and the earlier ones in `history`, each with a `tonepush-why.json` noting why it was set aside) and verified PRO rollback bundles |

On macOS these sit under `~/Library/Application Support`; on Windows, under your profile.

None of it is a lock-in. If you stop using TonePush tomorrow, your tones are still files you can open.
