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

Right-click a preset in the list and choose **Keep in your library** (Ctrl D) to copy it into your library. What goes in is the device's own document, byte for byte, so the snapshots and the routing come with it. Nothing is rebuilt from what the editor happens to show, which means nothing is quietly dropped. A check beside a preset means your library holds it unchanged; an orange compare mark means your library holds a different version under that name, and the same menu offers **Update in your library**.

Tones in your library are ordinary files in an ordinary folder. You can back them up, sync them, or read them with something else.

Every tone says which pedal it is for, in the **Pedal** column of the library and of the Cloud, beside a setlist's name, and in a tone's details: the family, then the model (**HX Stomp**, **HX Effects**, **HX Helix LT**) or, for a StompStation PRO, the firmware its chain needs (**PRO 2.x** when the preset has a chain layout, **PRO 1.5** when it has none). TonePush records the pedal a tone was kept from, and its firmware, when you keep it; for a tone kept before it did, it reads the tone itself, which tells a Stomp from an Effects or a Helix but not a Stomp from a Stomp XL. A tone the connected pedal cannot play has a dashed marker and a ban mark where the pedal mark would be, with the reason on hover: another family, another model (an HX Stomp XL reads HX Stomp tones, not the other way round), a tone made on a newer firmware release (patch releases count as the same release), or a PRO tone with a chain layout on a PRO still on 1.5. With the library scoped to the connected pedal, those tones are left out; **All pedals** shows them.

### Hear a tone with one click

A click on a tone in your library plays it on the pedal, in the place of the preset that is loaded. Nothing is written to the pedal's memory: the loaded preset is set aside with its unsaved changes and its undo history, and a bar along the top of the library says what plays and what waits. Click another tone, or press ↓ and ↑ to step through the list, and each one plays against that same preset; Space plays the chosen tone, or puts back the one playing. Ctrl-click and Shift-click choose tones without playing them.

- **Put back** (Esc) writes the preset back exactly as it was, changes and all.
- **Keep in 01B** (Enter, named after the loaded slot) makes what plays that slot's edit: Save then writes it over the preset, and Undo still brings the preset back. Its menu also offers **In another slot…** (Ctrl Enter): choose a slot on the left, and it is written once the preset set aside is back.

Turn its knobs while it plays and the changes stay with it, and the bar counts them: Keep keeps them, Put back drops them and restores the preset exactly as it was set aside, and stepping to another tone leaves them behind. Undo and redo work on those changes alone while it plays.

While a tone plays, Save is off and Ctrl S points at the bar instead; the deck, the chain and the preset's row in the sidebar say what is auditioned. Switching between the library's tabs leaves it playing. Clicking another preset puts it back first, then asks about its changes as any switch does, and clicking the loaded preset's own row puts it back. A tone the pedal cannot play says why where the bar would be, and offers to show only what the pedal plays. A StompStation PRO auditions the same way, and so does a tone from TonePush. A click while the pedal is busy, such as a StompStation PRO backing up, waits its turn and plays once the pedal is free; delete that tone first, or let the pedal go, and it is not played. A tone the pedal could not play says why, and the preset set aside is back.

### Put tones in the pedal's slots

The pedal mark at the start of a tone's row, **Put in a slot…** in its details, and Keep's **In another slot…** all turn the presets on the left into destinations: free slots read Empty in amber, and the row under the pointer says Replace or Put it here. Choose one. An empty slot is written at once. A slot that holds a preset asks first, beside it: what it replaces, whether that preset stays in your library and the pedal's latest backup, and what becomes of the preset that is loaded; **Replace 05B** (Enter) writes it, Esc or Cancel leaves the pedal as it was. Choose several tones with Ctrl-click or Shift-click and they go in a run from the slot you choose, one slot each in the order the list shows them, with one question that lists every slot when any of them holds a preset. The deck says what was written.

On a StompStation PRO with no checked backup yet, the question offers **Back up, then put it in 05B**: TonePush backs the pedal up, which takes about 40 seconds, and writes the slot once the backup is checked. On firmware TonePush has not verified for saving, nothing is written to a slot: Keep's menu says so, and Keep in the loaded slot still plays the tone until you load another preset.

### Drag what you see

Every drag is a shortcut for something a menu or a key also does, and the ghost under the pointer says what the drop will do, or why it will not: "Replace 05B Chime Clean", "The HX Stomp cannot play a StompStation PRO tone".

- **Tones from your library** onto a preset put them in that slot, several in a run from it; onto the board they play, as a click does; onto the **Cloud** tab one is published; onto a setlist's slot they make that setlist's next version, with the tones in it.
- **A TonePush tone** onto a preset is kept in your library and then put in that slot; onto the **Tones** tab it is kept; onto the board it plays.
- **A preset** onto the **Tones** tab is kept in your library; onto another preset it is copied there, and moved with Shift held; onto the **Cloud** tab it is kept, then published.
- **The preset's name** at the top of the window, onto the **Tones** tab, keeps it as it sounds now, edits and all.
- **A setlist** onto the presets puts the whole of it on the pedal, after the question that always asks. **One of its slots** onto a preset writes that one preset there; onto the **Tones** tab, keeps it in your library.
- **The Presets heading**, the whole pedal, onto the **Setlists** tab keeps it as a new setlist; onto a setlist, as that setlist's next version.
- **A tone file** from outside TonePush onto the **Tones** tab is imported; onto a preset, it is put in that slot.

While something is dragged, the presets read as destinations and the library's tabs say whether they take it, even folded. A drop that writes the pedal's memory asks first, the way putting a tone in a slot does. Esc lets go of a drag.

### Right-click, or press a key

Every row has a menu, and every menu has the same order: play it, put it on the pedal, keep it; then its name and versions; then what takes it out of this computer (publishing, its page on tonepush.rocks, exporting, its folder); delete last, in red. Right-click chooses a row without playing it. An item a row cannot use stays in the menu and says why at its right, such as **for PRO 2.x** beside Play for a tone the connected pedal cannot play.

- **A tone in your library**: Play on the HX Stomp, Put in a slot…, Rename, Versions, Publish on TonePush…, Open on tonepush.rocks, Export…, Show in folder, Delete…. Choose several and the menu puts them in slots in a run, publishes them one after another, exports them into one folder, or deletes them.
- **A setlist**: Put on the HX Stomp…, Capture the pedal as its next version, Rename, Versions, Export… (a folder of its tones, one file a slot, named for the slot), and Delete…, which asks and takes only that version: the other versions stay, and so do the tones it plays.
- **One slot of a setlist**: play it, Send to 05A, Put in another slot…, and Show in your tones.
- **A preset on the pedal**: Rename, Copy, Paste, Keep in your library, Publish on TonePush… (kept in your library first, then published from it), Save to file…, Load from file…, favourites, and Empty this slot….
- **A tone on TonePush**: play it, Put in a slot… (kept in your library first), Keep in your library, Versions, Open on tonepush.rocks, and Copy link. Someone else's tone has nothing to rename or delete.

The keys beside the items work without opening the menu, on the list you clicked last:

| Key | What it does |
|---|---|
| Space | Play the chosen tone, or put back the one playing |
| ↑ ↓ | Step through the list, playing each tone; on the presets, load the next one |
| Enter | While a tone plays, keep it in the loaded slot |
| Ctrl Enter | Put the chosen tones in a slot; while a tone plays, in another slot |
| Esc | Put back what plays; otherwise close what is open |
| F2 | Rename the chosen tone, setlist or loaded preset |
| Del | Delete the chosen tones, or the chosen setlist's version, after asking |
| Ctrl D | Keep the chosen TonePush tone, or the loaded preset, in your library |
| Ctrl C, Ctrl V | Copy the loaded preset; paste what was copied over it |
| Shift F10 | Open the chosen row's menu |
| Ctrl L | Fold the library, or open it |
| Ctrl 1, 2, 3 | Show Tones, Setlists or Cloud |
| Ctrl F | Search the open tab |

### 3. Build a setlist

Get the pedal holding the presets you want, in the order you want them, then click the computer icon at the top of the preset list (**Keep every preset on the pedal, in order, as a setlist**), or **Keep the whole pedal as a setlist** on the library's Setlists tab. That records every slot and what is in it; name it, and it opens on that tab.

Give it the name of the gig, the venue, the date. You will want them later; click any of them on the setlist's page to change it.

### 4. Play it back

The library's Setlists tab shows each setlist against what is on the pedal now: its card says whether it matches the pedal or how many slots differ, and its page lays the slots out bank by bank, marking the ones that hold another preset and naming the ones it would empty. The switch beside the count narrows the banks to the slots that differ.

**Put on HX Stomp…** (named after your pedal) writes the whole thing back. It asks first, and says what the write changes: the presets it replaces, the slots it empties and fills, and how many already match. It writes every slot, in order, so tick **Keep the pedal as it is now as a setlist first** if you want what is on it now as a setlist too; nothing is written until that copy is in your library. On an HX pedal the pedal as it was is also kept in its backups (see below).

A click on a slot plays the setlist's version of it, wherever the pedal's own copy is, with the same bar as a click in your library: **Send to 05A** writes it back into its slot, asking first when that replaces a preset; the slot's menu has it too.

## Changing a setlist

Put it back on the pedal, edit there, keep the changed tones to your library, and capture a new setlist.

A setlist is never edited in place. That looks like a limitation and is not: a setlist is a record of a rig that worked on a particular night, and a record you can edit is not a record. Renaming a tone next month should not reach backwards and change what you played in March.

This is also why deleting a tone from your library never breaks a setlist. If a setlist still plays it, the tone is kept for that setlist even after it leaves your library. Deleting a setlist takes one version at a time, and asks first.

A setlist can still grow a new version without the pedal: drop tones from your library into its slots, and TonePush saves the setlist with them as its next version, leaving the one before as it was.

## Backups

Every time an HX pedal connects, TonePush reads all of it, presets, impulse responses and settings, and keeps that copy current after every save. Before it reads the pedal again, it sets the copy it had aside: when the pedal connects, before a setlist or presets are written to it, and before a restore. The last ten are kept.

**Pedal, Backups** lists them, newest first, with when and why each was taken, beside the backups you saved with **Back up to a file…** (TonePush remembers where the last twenty went, in its settings file). Choose one to compare it with the pedal now, preset by preset ("Ambient Swell edited since", "Sparkle Verb was Night Verb"). **Restore the whole pedal…** writes it back, presets, impulse responses and settings, after saying what it writes; the pedal as it was stays in the backups. **Show in folder** opens the copy itself. The StompStation PRO keeps its own verified backups; see the [StompStation PRO guide](/stompstation-pro/).

## Publishing a Song and Tone

On [TonePush](https://tonepush.rocks), a **Song** is the musical idea: either a catalog song by an Artist or an original. A **Tone** is one playable, device-native preset belonging to that Song.

**Publish on TonePush…** (in a tone's menu, its details, or a drop on the **Cloud** tab) asks first. The sheet names the Song, the Tone, the pedal and firmware it is for, who can see it and the account it goes up under, and for a tone already on TonePush the version it becomes: "TonePush has v2 of this tone, downloaded 312 times. v3 becomes the version people get; v2 stays on its page and can still be downloaded." Not signed in, the sheet signs you in first. The Tone is listed for the pedal it was kept from and the firmware it was made on, whatever pedal is connected when you publish it.

The first time, publishing creates the Song and then attaches the publishable device artifact (`.hlx` for Line 6 or `.vxpreset` for StompStation PRO) as its first Tone; if adding the Tone fails, the editor says that the empty Song remains instead of pretending the two requests were one transaction. TonePush's answer is kept in your library (`published.json`), so a later version of the same tone goes to the same Tone under the same Song, as its next version, rather than starting a new Song each time; if TonePush no longer has that Tone, the next publish starts it over. Tones published before the editor kept these records are found once, by name and the exact file, in TonePush's feed. Choose several tones and **Publish 3 on TonePush…** publishes them one after another, stopping at the first that fails. A preset on the pedal is kept in your library first and published from there.

**Cloud, Mine** lists what this library published: each tone's pedal, its Song, the version TonePush gives ("v2 · library has v3" when your library has moved on), its downloads and when it went up, with its versions on TonePush in the details, a click on one playing it. Its menu publishes the library's version, renames it on TonePush, and makes an earlier version current again; the last two publish its file again, which is how TonePush changes a name or the version people get without a new version. **Open on tonepush.rocks** and **Copy link** take you to its page. The keys work there too: the arrows step through it playing each, Space plays or puts back, Ctrl Enter puts one in a slot and F2 renames it on TonePush.

Where TonePush can list your account (tonepush.rocks can; an older server or a `TONEPUSH_SITE` pointed elsewhere may not, and then Mine stays as above), Mine lists everything you published from any computer, your setlists first, each with who can see it: **Everyone**, or **Only you**. **Hide** and **Show** in a row's details or menu switch that; renaming changes only the name, with no new upload; **Delete from TonePush…** asks first, with what goes (its page and versions), what stays (the copies people downloaded, and the tone in your library), and the gentler choice of hiding it. The publish sheet then offers Only you as well, and a setlist's menu offers **Publish on TonePush…**: on TonePush a setlist is a list of your published tones, each the version the setlist holds, so the sheet names any tone that is not on TonePush yet. **Your page on tonepush.rocks** is in Mine's head and in the account's menu. What the server cannot do is left out of the menus rather than failing when clicked.

**Export for the web** writes the same information without publishing it: the
Tone's `.hlx` or `.vxpreset`, plus a `.json` manifest with separate `song` and
`tone` objects. Song facts include its title, kind, Artist, description and
tags; Tone facts include the preset name, part, guitar, tuning and
device-specific description.

Song search results are musical ideas, not files that can be installed. Open a Song and choose one of its Tones for your device before downloading or installing it. An externally indexed Tone opens its original source; a native Tone downloads its hosted artifact.

In the library's **Cloud**, a click on a Tone plays it on the pedal, the way a click on a tone in your library does: the bar says Loading while its file comes, then Playing. Each file is fetched once a session, and the arrow keys step through the list, playing only the Tone you stop on. TonePush counts a play as an audition, apart from its downloads; keeping a Tone counts as a download. **Keep in library** (Ctrl D) keeps it without touching the pedal, and **Keep in 01B** keeps it on the pedal and in your library too, so the pedal never holds a Tone your library does not know. The details list a Tone's versions: a click on one plays it. A Tone hosted by another catalog, such as Line 6 CustomTone, is not played from here: the bar says where it lives, with **Open**. The versions of a tone in your library play the same way.

With no pedal connected, a click on a tone in your library shows it where the editor would be, read only: its chain, and the faces of its blocks, with **Back to Plug in your pedal** (Esc) to return. A StompStation PRO tone is only described until its pedal is connected, since drawing its chain needs the pedal.

## Where everything is

| | Where | What it is |
|---|---|---|
| Tones | `~/.local/share/tonepush/library` | One file per tone, the pedal's own document |
| Setlists | `library/setlists` | Small JSON files naming the tones they play |
| What you published | `library/published.json` | Which Tone and Song each tone is on TonePush, so its next version goes there |
| Device backups | `~/.local/share/tonepush/backups` | Whole-pedal HX backups (the copy kept current, and the earlier ones in `history`, each with a `tonepush-why.json` noting why it was set aside) and verified PRO rollback bundles |

On macOS these sit under `~/Library/Application Support`; on Windows, under your profile.

None of it is a lock-in. If you stop using TonePush tomorrow, your tones are still files you can open.
