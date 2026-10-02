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
