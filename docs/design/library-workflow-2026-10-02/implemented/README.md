# The library workflow as built

Screenshots of the editor itself, for comparing the implementation with the
approved mockups beside this folder (`../README.md`). `before/` is the editor
as the work started (0.8.0's frame, with the Edit · Library · Pedal switch);
the files here are the editor after the latest stage, named
`<scene>-<width>x<height>-<theme>.png`. Each stage's commit holds the
screenshots as that stage left them.

They are drawn by the same offscreen harness as the redesign's
(`src/screenshots.rs` in `tonepush-gui`, described in
`../../redesign-2026-10-01/implemented/README.md`), on the GPU under
`gpu-lock`, with invented content only: the tones, setlists, songs, artists
and TonePush creators are the design's own (`../source/assets/data-lib.js`),
no pedal is opened, nothing is sent to TonePush's site (any request goes to
`http://127.0.0.1:9`), and the data directories are scratch copies.

```sh
TONEPUSH_SCREENSHOTS=out \
TONEPUSH_LIBRARY=scratch/library TONEPUSH_BACKUPS=scratch/backups \
TONEPUSH_CONFIG=scratch/config.json HX_RESOURCES_DEST=scratch/hx-resources \
TONEPUSH_SITE=http://127.0.0.1:9 \
gpu-lock cargo test -p tonepush-gui --lib screenshots -- --ignored --test-threads=1
```

## Before

The HX Stomp's Edit page at the three sizes in both themes (`hx-edit`), and at
1280 × 760: the Library page's tones (`hx-library`, also light) and setlists
(`hx-setlists`), the HX's Pedal page (`hx-pedal`), the StompStation PRO's Edit
page (`pro-edit`), the PRO on firmware it has not verified for saving
(`pro-read-only`), its NAM amps (`pro-nam`), and the window with no pedal
(`no-device`).

## Stage 1: device markers

Every tone row says which pedal it is for, in a Pedal column after the place
marks, on Tones and Cloud alike: the family, then the model, or for a
StompStation PRO the firmware its chain needs (2.x with a chain layout, 1.5
without). It is solid when the connected pedal plays the tone; dashed and
faint when it does not, with a ban mark in the pedal's place and the reason on
hover. The smallest window keeps the family alone. Setlists carry the marker
beside their name, on the card and the page, and a tone's details lead with
it.

The library records the pedal a tone was kept from, and its firmware, when it
keeps it: from the pedal's menu, a capture of the whole pedal, an update from
the pedal, or the Cloud (the device TonePush lists). A tone kept before this is
read: an HX document's structure says Stomp, Effects or Helix, an `.hlx` names
its device, and a PRO preset with a chain layout needs 2.x. Publishing lists a
tone for that pedal and that firmware, not the pedal connected.

Scenes: `hx-library` (every pedal's tones, at the three sizes in both themes),
`hx-cloud` (new: TonePush's tones for the HX Stomp) and `hx-setlists`. Their
`before/` is `before/hx-library` and `before/hx-setlists`.

- An HX document cannot tell a Stomp from a Stomp XL, nor one Helix from
  another: its endpoints carry no model, so a tone kept before the pedal was
  recorded reads as HX Stomp (which an XL also plays) or Helix. Every Helix
  plays the others' tones, since they share a preset format; the sheet lists
  only the Floor's.
- Firmware is compared by release, major and minor, as decided for the PRO:
  1.5.10 is 1.5.12. Line 6 writes its versions with the patch as the last
  digit (3.81 is 3.8, patch 1), so 3.81 is 3.80 too.
- A TonePush PRO tone that lists no firmware keeps the family alone in its
  marker and plays on any PRO.

## Stage 2: the frame

The Edit · Library · Pedal switch is gone. The library is a pane along the
bottom of the editor with three tabs (Tones, Setlists, Cloud, each with its
count), the Cloud's Everyone and Mine beside them, and the open tab's search,
filter, pedal scope, columns and account at the right. Its top edge drags it
taller or shorter, a double-click or Ctrl L folds it to its tabs, Ctrl 1, 2 and
3 open its tabs and Ctrl F searches the open one. It opens at the design's
heights (186, 232 and 512 points for the three sizes of window), and its
height and whether it is folded are kept for each size. With nothing chosen
the details are the loaded preset's tone, in full, and the table scrolls to
its row. The block pane gives way as the library grows: the face on one row
that scrolls sideways, then the block's head alone; the footswitches show
when the library is folded or leaves room for them.

The pedal's own pages open from its card at the top of the sidebar, or from
the menu its arrows open, which lists them with their counts and ends with
"Let the pedal go". "Back to the preset" (Esc) returns to the editor, and the
library starts folded there with its key beside it. The connect page scrolls
when the pane leaves it short.

Scenes: `hx-edit` (the main screen, at the three sizes in both themes,
against sheet 01), `hx-library` (the three sizes in both themes), `hx-cloud`,
`hx-setlists`, `hx-pedal` and `hx-pages` (sheet 21, the card's menu open),
`pro-edit`, `pro-pages` (sheet 22, a PRO on 2.0.10 on its NAM amps) and
`no-device`, at 1280 × 760. Their `before/` is the scene of the same name, and
`before/hx-library` for `hx-cloud`.

- The pane's account is signed in under an invented name ("Noa Calder") and
  no token, so nothing could reach an account.
- At the middle size the Cloud tab shows the account as an icon, as the
  design does, so the tabs keep their room; any tool that still does not fit
  narrows the search, which is left out below 96 points.
- The design's largest window also starts with the Version and Changed
  columns; the table keeps its existing first columns, and both are on its
  columns menu.
- The Cloud table keeps its existing columns until stage 6 restyles TonePush's
  tones (By and Downloads in place of Character and Rating).

## Stage 3: a click plays a library tone

A click on a tone in the library plays it on the pedal, in the loaded
preset's edit buffer; Ctrl-click and Shift-click only choose. The first one
sets the loaded preset aside with its unsaved changes and its undo history,
and every one after it plays against that baseline. The audition bar on the
pane's top edge says what plays and what waits, with the arrow keys that step
(the worker plays only the row stopped on), **Put Plexi Crunch back** (Esc)
and **Keep in 01B** (Enter), whose menu adds **In another slot…** (Ctrl
Enter). The deck names the tone ("Auditioning, from your library · Plexi
Crunch is set aside") with Save off, Ctrl S pointing at the bar instead; the
board has its amber rule and "Auditioning Dream Pop"; the row is tinted with
a speaker for its pedal mark, as is the loaded preset's row in the sidebar;
the details say "Playing in 01B, in place of Plexi Crunch" and where else the
tone is. Space plays the chosen tone or puts back the one playing; switching
the library's tabs leaves it playing; another preset puts it back first, then
asks; the loaded preset's own row puts it back. A tone the pedal cannot play
says why in the bar's place, in the info voice, with "Show only what the HX
Stomp plays". The StompStation PRO auditions the same way, and TonePush's
tones, which already played on a click, now use the same bar.

Scenes: `hx-audition` (sheet 02, at the three sizes in both themes),
`hx-audition-keep` (sheet 03), `hx-cannot-play` (sheet 05) and
`pro-audition`, at 1280 × 760, with the earlier scenes drawn again.

- The large window's "In your library" card is gone, as the design says: the
  details beside the table are that card now. A loaded preset the library does
  not hold yet offers "Keep in library" there.
- The design writes "Plexi Crunch waits, with its 3 unsaved changes"; the
  editor does not count changes anywhere else, so the bar says "with its
  unsaved changes".
- An edit made during an audition still ends it, as before; stage 4 keeps it
  in the audition. The menu's note says another slot is written once chosen;
  stage 5 adds the question before it is.

## Stage 4: edits during an audition stay in it

Turning a knob, switching a snapshot or undoing while a tone is auditioned
no longer ends the audition. The HX worker puts an audition back only before
what leaves the loaded preset or writes the pedal's memory (another preset,
Save, a slot or setlist written, a backup or restore, letting the pedal go);
an edit stays with the tone, with an undo history of its own. Put back still
writes the set-aside preset back byte for byte with its history, Keep keeps
what is heard with the set-aside preset one undo step under it, and stepping
to another tone drops the edits made to the last. On a StompStation PRO the
worker sets the undo history aside the same way and remembers the value every
edit replaces, the chain's included, so Put back restores those too; a save
that arrives during an audition saves the preset's own changes. The bar
counts the edits ("Dream Pop is playing in 01B, with 2 changes"), and a
switch to another preset asks only about the set-aside preset's changes.

Scene: `hx-audition-edited`, at 1280 × 760.
