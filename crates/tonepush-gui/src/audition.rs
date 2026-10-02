//! Hearing a tone on the pedal with one click, and the two ways out of it.
//!
//! A click on a tone plays it in the loaded preset's edit buffer. The first
//! one sets the loaded preset aside, with its unsaved changes and its undo
//! history; every one after it plays against that same baseline. Put back
//! and Keep are the only ways out: Put back writes the preset back byte for
//! byte, Keep makes what plays the loaded slot's edit, which Save then
//! writes. The pedal's workers hold what was set aside (`session.rs`'s
//! `Audition`, the StompStation PRO's `audition_original`); this holds what
//! the editor asked to hear and what it set aside, and draws the bar on the
//! library pane's top edge that says so, or says why a click did not play.

use egui::{Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::library;
use crate::session::Cmd;
use crate::shell;
use crate::theme::{self, Icon, Tier};
use crate::{App, LibEntry, LibraryView};

/// The bar's height, on the library pane's top edge.
pub(crate) const BAR: f32 = 46.0;

/// Library tones are played under keys from here up, clear of TonePush's
/// tone ids and of the negative keys its older versions are cached under.
const LIBRARY_KEYS: i64 = 1 << 48;

/// How long Ctrl S points at the bar, in seconds.
const NUDGE: f64 = 0.9;

/// Where a tone being heard came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// A tone in the library, by its hash.
    Library(String),
    /// A tone on TonePush, by the key its file is cached under: its id, or
    /// for an older version the negative key `cloud_version_entry` gives it.
    TonePush(i64),
    /// A setlist's slot: the tone it plays, the setlist's name and the slot.
    Setlist {
        hash: String,
        setlist: String,
        slot: i64,
    },
}

impl Source {
    /// Where it came from, as the deck says it: "Auditioning, from your
    /// library".
    pub(crate) fn words(&self) -> String {
        match self {
            Source::Library(_) => "from your library".to_owned(),
            Source::TonePush(_) => "from TonePush".to_owned(),
            Source::Setlist { setlist, .. } => format!("from {setlist}"),
        }
    }

    /// The library tone it plays, when it is one.
    pub(crate) fn hash(&self) -> Option<&str> {
        match self {
            Source::Library(hash) | Source::Setlist { hash, .. } => Some(hash),
            Source::TonePush(_) => None,
        }
    }
}

/// A tone asked to play, under the key the pedal's worker plays it by.
#[derive(Clone, Debug)]
pub(crate) struct Heard {
    pub key: i64,
    pub source: Source,
    pub name: String,
}

/// The preset the first audition set aside: its slot, its name, and whether
/// it had changes not saved.
#[derive(Clone, Debug, Default)]
pub(crate) struct SetAside {
    pub slot: String,
    pub name: String,
    pub dirty: bool,
}

/// Why a click did not play, said where the bar goes. Not an error: the
/// same place every click reports to.
#[derive(Clone, Debug)]
pub(crate) struct Strip {
    pub icon: Icon,
    /// The words, a run of plain and bold.
    pub words: Vec<(String, bool)>,
    /// The pedal the library could be narrowed to, when it shows every
    /// pedal's tones.
    pub narrow: Option<String>,
    /// Where it lives instead, for a tone hosted elsewhere: Open goes there.
    pub open: Option<String>,
}

impl Strip {
    fn plain(icon: Icon, words: impl Into<String>) -> Strip {
        Strip {
            icon,
            words: vec![(words.into(), false)],
            narrow: None,
            open: None,
        }
    }
}

/// A tone shown, not played, while no pedal is connected: the editor shows
/// its chain and faces read only, where the connect page was.
pub(crate) struct Glimpse {
    pub preview: crate::Preview,
    pub marker: Option<crate::devices::Marker>,
    /// Where it came from: "From your library".
    pub from: String,
}

/// Which list the arrow keys step through: the last one clicked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Arrows {
    /// The pedal's presets, as built.
    #[default]
    Presets,
    /// The library's tones, as the table draws them.
    Tones,
    /// TonePush's tones, as the Cloud draws them.
    Cloud,
}

/// What the deck, the board and the sidebar say while a tone is auditioned.
#[derive(Clone, Debug, Default)]
pub(crate) struct Shown {
    /// The tone playing, or on its way.
    pub name: String,
    /// Where it came from: "from your library".
    pub from: String,
    /// From somewhere the board names: TonePush, or a setlist.
    pub named: bool,
    /// What the deck says second, in place of what was set aside: a
    /// setlist's "its 05A".
    pub aside: Option<String>,
    /// Still on its way to the pedal.
    pub loading: bool,
    /// The preset set aside, and whether it had changes not saved.
    pub set_aside: String,
    pub set_aside_dirty: bool,
}

impl Shown {
    /// The deck's second line while it plays.
    pub(crate) fn state(&self, tier: Tier) -> Vec<shell::State> {
        let mut parts = vec![if self.loading {
            shell::State::Busy(format!("Loading, {}", self.from))
        } else {
            shell::State::Audition(format!("Auditioning, {}", self.from))
        }];
        if tier != Tier::S {
            if let Some(aside) = &self.aside {
                parts.push(shell::State::Note(aside.clone()));
            } else if !self.set_aside.is_empty() {
                parts.push(shell::State::Note(format!(
                    "{} is set aside",
                    self.set_aside
                )));
            }
        }
        parts
    }

    /// What the board's header says after its own words.
    pub(crate) fn board_note(&self) -> String {
        if self.named {
            format!("Auditioning {}, {}", self.name, self.from)
        } else {
            format!("Auditioning {}", self.name)
        }
    }
}

/// The audition as the editor holds it.
#[derive(Default)]
pub(crate) struct Hearing {
    /// The tone asked to play, the latest.
    pub heard: Option<Heard>,
    /// What the first audition set aside.
    pub set_aside: Option<SetAside>,
    /// Why the last click did not play.
    pub strip: Option<Strip>,
    pub arrows: Arrows,
    /// When Ctrl S last pointed at the bar, in the context's time.
    pub nudged: Option<f64>,
    /// A tone shown read only while no pedal is connected.
    pub glimpse: Option<Glimpse>,
    /// The board and the face are drawing that tone this frame.
    pub glimpsing: bool,
    next: i64,
}

impl Hearing {
    fn next_key(&mut self) -> i64 {
        self.next += 1;
        LIBRARY_KEYS + self.next
    }
}

impl App {
    /// Whether an audition is going on: a tone asked to play, or one the
    /// pedal plays.
    pub(crate) fn hearing(&self) -> bool {
        self.hearing.heard.is_some() || self.auditioning.is_some()
    }

    /// Whether the tone asked to play is still on its way to the pedal.
    pub(crate) fn heard_loading(&self) -> bool {
        self.hearing
            .heard
            .as_ref()
            .is_some_and(|heard| self.auditioning != Some(heard.key))
    }

    /// The tone the pedal plays, once it plays it.
    pub(crate) fn heard_playing(&self) -> Option<&Heard> {
        self.hearing
            .heard
            .as_ref()
            .filter(|heard| self.auditioning == Some(heard.key))
    }

    /// Whether this tone is the one asked to play.
    pub(crate) fn hears(&self, source: &Source) -> bool {
        self.hearing
            .heard
            .as_ref()
            .is_some_and(|heard| &heard.source == source)
    }

    /// The library tone playing, or asked to play, whichever list it came
    /// from: its row in the library is marked too.
    pub(crate) fn heard_hash(&self) -> Option<&str> {
        self.hearing
            .heard
            .as_ref()
            .and_then(|heard| heard.source.hash())
    }

    /// Whether the bar, or the strip in its place, is on the pane.
    pub(crate) fn bar_shown(&self) -> bool {
        self.hearing.heard.is_some() || self.hearing.strip.is_some()
    }

    /// What the deck, board and sidebar say while a tone is auditioned.
    pub(crate) fn shown_hearing(&self) -> Option<Shown> {
        let heard = self.hearing.heard.as_ref()?;
        let set_aside = self.hearing.set_aside.clone().unwrap_or_default();
        Some(Shown {
            name: heard.name.clone(),
            from: heard.source.words(),
            named: !matches!(heard.source, Source::Library(_)),
            aside: match &heard.source {
                Source::Setlist { slot, .. } => {
                    Some(format!("its {}", self.active_slot_label(*slot)))
                }
                _ => None,
            },
            loading: self.heard_loading(),
            set_aside: set_aside.name,
            set_aside_dirty: set_aside.dirty,
        })
    }

    /// The loaded preset, as an audition sets it aside.
    fn loaded_preset(&self) -> SetAside {
        if self.pro_active() {
            SetAside {
                slot: self
                    .pro
                    .loaded_slot()
                    .map(crate::pro::slot_label)
                    .unwrap_or_default(),
                name: self.pro.loaded_name(),
                dirty: self.pro.is_dirty(),
            }
        } else {
            SetAside {
                slot: if self.preset_index >= 0 {
                    self.active_slot_label(self.preset_index)
                } else {
                    String::new()
                },
                name: self.preset_name.trim().to_owned(),
                dirty: self.dirty,
            }
        }
    }

    /// Note a tone asked to play. The first one sets the loaded preset
    /// aside; the ones after it play against that same baseline.
    pub(crate) fn begin_hearing(&mut self, key: i64, source: Source, name: String) {
        if !self.hearing() {
            self.hearing.set_aside = Some(self.loaded_preset());
        }
        self.hearing.strip = None;
        self.hearing.heard = Some(Heard { key, source, name });
    }

    /// Forget the tone asked to play and what it set aside.
    pub(crate) fn forget_heard(&mut self) {
        self.hearing.heard = None;
        self.hearing.set_aside = None;
        self.hearing.nudged = None;
    }

    /// Why a click on this library tone cannot play it, in the strip's
    /// words, or nothing when it can.
    pub(crate) fn cannot_play(&self, entry: &LibEntry) -> Option<Strip> {
        if !self.pedal_online() {
            return Some(Strip {
                icon: Icon::Usb,
                words: vec![
                    ("No pedal is connected, so ".to_owned(), false),
                    (entry.name.clone(), true),
                    (
                        " is not played. Plug in a pedal and the next click plays.".to_owned(),
                        false,
                    ),
                ],
                narrow: None,
                open: None,
            });
        }
        if self.pro_active() && self.pro.updating() {
            return Some(Strip::plain(
                Icon::Info,
                "Auditions wait for the firmware update.",
            ));
        }
        let why = self.refusal_of(entry).or_else(|| {
            (!self.tone_kind_compatible(&entry.hash))
                .then(|| format!("{} is for another kind of pedal.", entry.name))
        })?;
        let loaded = self.loaded_preset().name;
        let mut words = Vec::new();
        let rest = match why.strip_prefix(entry.name.as_str()) {
            Some(rest) => {
                words.push((entry.name.clone(), true));
                rest.to_owned()
            }
            None => why,
        };
        // "The HX Stomp cannot play it, so it stays on Plexi Crunch."
        let rest = match rest.strip_suffix("cannot play it.") {
            Some(start) if !loaded.is_empty() => {
                format!("{start}cannot play it, so it stays on {loaded}.")
            }
            _ => rest,
        };
        words.push((rest, false));
        let narrow = (self.library_device_filter.is_none()
            && !self.library_connected_device.is_empty())
        .then(|| self.library_connected_device.clone());
        Some(Strip {
            icon: Icon::Info,
            words,
            narrow,
            open: None,
        })
    }

    /// Play a library tone, by its row, in the loaded preset's edit buffer.
    /// A tone that cannot play says why where the bar goes, and stays
    /// chosen so its details show.
    pub(crate) fn audition_library(&mut self, index: usize) {
        let Some(entry) = self.lib_entries.get(index).cloned() else {
            return;
        };
        self.audition_stored(Source::Library(entry.hash.clone()), &entry);
    }

    /// Play one of a library tone's older versions: the tone's row says
    /// which pedal plays it, the version's bytes are what plays.
    pub(crate) fn audition_version(&mut self, index: usize, hash: String, number: u32) {
        let Some(mut entry) = self.lib_entries.get(index).cloned() else {
            return;
        };
        entry.name = format!("{} v{number}", entry.name);
        entry.hash = hash.clone();
        self.audition_stored(Source::Library(hash), &entry);
    }

    /// Play a setlist's slot: the setlist's version of the tone, wherever
    /// the pedal's own copy of that slot is.
    pub(crate) fn audition_setlist_slot(&mut self, setlist: &str, slot: i64, tone: &library::Slot) {
        let entry = self.entry_for_hash(&tone.hash, &tone.name);
        self.audition_stored(
            Source::Setlist {
                hash: tone.hash.clone(),
                setlist: setlist.to_owned(),
                slot,
            },
            &entry,
        );
    }

    /// A library tone as a row, for one that is not a row of its own: a
    /// setlist's tone the library has let go, or an older version. What
    /// the library recorded about it says which pedal plays it.
    fn entry_for_hash(&self, hash: &str, name: &str) -> LibEntry {
        if let Some(entry) = self.lib_entries.iter().find(|entry| entry.hash == hash) {
            return entry.clone();
        }
        let meta = library::meta_of(hash).unwrap_or_default();
        let marker = (!meta.pedal.is_empty())
            .then(|| crate::devices::Marker::of_device(&meta.pedal, &meta.firmware))
            .flatten();
        LibEntry {
            hash: hash.to_owned(),
            series: hash.to_owned(),
            name: name.to_owned(),
            line: String::new(),
            firmware: meta.firmware.clone(),
            meta,
            added_at: String::new(),
            modified_at: String::new(),
            downloads: None,
            rating: None,
            version: 1,
            versions: 1,
            chain: Vec::new(),
            pro: library::kind(hash).as_deref() == Some("vxpreset"),
            marker,
        }
    }

    /// Play a tone the library stores, as the source says it came.
    fn audition_stored(&mut self, source: Source, entry: &LibEntry) {
        if self.hears(&source) {
            return;
        }
        self.hearing.strip = None;
        if !self.pedal_online() {
            return self.glimpse(entry);
        }
        if let Some(strip) = self.cannot_play(entry) {
            self.hearing.strip = Some(strip);
            return;
        }
        let Some(bytes) = library::read(&entry.hash) else {
            return self.problem(format!("{} is missing from the library", entry.name));
        };
        let key = self.hearing.next_key();
        let name = entry.name.clone();
        match library::kind(&entry.hash).as_deref() {
            Some("vxpreset") => self.pro.audition(key, name.clone(), bytes),
            Some("hlx") => match self.steps_from_hlx(&name, &bytes) {
                Ok((_, blocks)) => self.send(Cmd::AuditionSteps {
                    key,
                    name: name.clone(),
                    blocks,
                }),
                Err(why) => return self.problem(why),
            },
            _ => self.send(Cmd::AuditionDocument {
                key,
                name: name.clone(),
                bytes,
            }),
        }
        self.begin_hearing(key, source, name);
    }

    /// With no pedal, a click shows the tone where the editor would be,
    /// read only, and the strip says what would make it play. A StompStation
    /// PRO tone is only described: drawing its chain needs the pedal's own
    /// schema.
    fn glimpse(&mut self, entry: &LibEntry) {
        let pedal = entry
            .marker
            .as_ref()
            .map_or_else(|| "pedal".to_owned(), |marker| marker.device_name());
        if entry.pro {
            self.hearing.glimpse = None;
            self.hearing.strip = Some(Strip {
                icon: Icon::Usb,
                words: vec![
                    ("No pedal is connected, so ".to_owned(), false),
                    (entry.name.clone(), true),
                    (
                        format!(
                            " is not played. Its chain is drawn once a {pedal} is connected, \
                             and the next click plays."
                        ),
                        false,
                    ),
                ],
                narrow: None,
                open: None,
            });
            return;
        }
        let Some(bytes) = library::read(&entry.hash) else {
            return self.problem(format!("{} is missing from the library", entry.name));
        };
        let before = self.preview.take();
        if library::kind(&entry.hash).as_deref() == Some("hlx") {
            self.preview_hlx(&entry.name, bytes);
        } else {
            self.preview_hxpreset(&entry.name, bytes);
        }
        let preview = std::mem::replace(&mut self.preview, before);
        self.hearing.glimpse = preview.map(|preview| Glimpse {
            preview,
            marker: entry.marker.clone(),
            from: "From your library".to_owned(),
        });
        let shown = if self.hearing.glimpse.is_some() {
            "shown, not played"
        } else {
            "not played"
        };
        self.hearing.strip = Some(Strip {
            icon: Icon::Usb,
            words: vec![
                ("No pedal is connected, so ".to_owned(), false),
                (entry.name.clone(), true),
                (
                    format!(" is {shown}. Plug in your {pedal} and the next click plays."),
                    false,
                ),
            ],
            narrow: None,
            open: None,
        });
    }

    /// Leave a tone shown with no pedal: the connect page comes back.
    pub(crate) fn close_glimpse(&mut self) {
        self.hearing.glimpse = None;
        self.hearing.strip = None;
    }

    /// Put back what the audition set aside, its changes and history with
    /// it.
    pub(crate) fn put_back(&mut self) {
        if !self.hearing() {
            return;
        }
        self.end_audition();
        self.forget_heard();
    }

    /// Keep what plays in the loaded slot, as its edit: Save then writes it
    /// over what was set aside, and Undo brings that back. A TonePush tone
    /// kept on the pedal goes into the library too, so the pedal never holds
    /// a tone the library does not know.
    pub(crate) fn keep_heard(&mut self) {
        let Some(heard) = self.heard_playing().cloned() else {
            return;
        };
        if let Source::TonePush(key) = heard.source {
            self.keep_cloud_heard(key);
        }
        let slot = self
            .hearing
            .set_aside
            .as_ref()
            .map(|set_aside| set_aside.slot.clone())
            .unwrap_or_default();
        self.keep_audition();
        self.note(format!("kept {} in {slot}", heard.name));
        self.forget_heard();
    }

    /// Keep what plays in another slot: the presets become destinations, the
    /// audition plays on while one is chosen, and is put back before that
    /// slot is written.
    pub(crate) fn keep_heard_elsewhere(&mut self) {
        let Some(heard) = self.heard_playing().cloned() else {
            return;
        };
        if self.elsewhere_refusal().is_some() {
            return;
        }
        let hash = match heard.source {
            Source::Library(hash) | Source::Setlist { hash, .. } => Some(hash),
            // Into the library first, so the slot is written from it.
            Source::TonePush(key) => self.keep_cloud_heard(key),
        };
        if let Some(hash) = hash {
            match self.lib_entries.iter().position(|entry| entry.hash == hash) {
                Some(row) => self.start_putting(&[row]),
                None => {
                    // A setlist's tone the library let go: still its own.
                    self.sending = Some(crate::put::Sending {
                        tones: vec![(hash, heard.name.clone())],
                    });
                }
            }
        }
    }

    /// Why Keep's "In another slot…" cannot be offered now, in the menu's
    /// few words: firmware TonePush only plays, or a PRO with no checked
    /// backup yet.
    pub(crate) fn elsewhere_refusal(&self) -> Option<String> {
        if let Some(why) = self.put_refusal() {
            return Some(why);
        }
        self.put_needs_backup()
            .then(|| "waits for a backup".to_owned())
    }

    /// Keep the TonePush tone heard under this key in the library, from the
    /// file already fetched for it. Answers its hash once it is there.
    pub(crate) fn keep_cloud_heard(&mut self, key: i64) -> Option<String> {
        let bytes = self.cloud_artifacts.get(&key).cloned()?;
        let entry = self.cloud_entry_for(key)?;
        self.apply_cloud_artifact(&entry, crate::CloudAction::Computer, bytes.clone());
        let hash = library::hash_of(&bytes);
        library::meta_of(&hash).map(|_| hash)
    }

    /// Step to the next or the previous tone in the list as it is drawn,
    /// and play it.
    pub(crate) fn step_tones(&mut self, direction: i64) {
        let order = self.lib_order.clone();
        if order.is_empty() || direction == 0 {
            return;
        }
        let at = self
            .lib_selected
            .and_then(|selected| order.iter().position(|&row| row == selected));
        let next = match at {
            Some(at) => (at as i64 + direction).clamp(0, order.len() as i64 - 1) as usize,
            None if direction > 0 => 0,
            None => order.len() - 1,
        };
        if Some(next) == at {
            return;
        }
        let index = order[next];
        self.choose_tone(index);
        self.lib_reveal_near = true;
        self.audition_library(index);
    }

    /// Choose one tone, as a click on its row does.
    pub(crate) fn choose_tone(&mut self, index: usize) {
        let Some(hash) = self.lib_entries.get(index).map(|entry| entry.hash.clone()) else {
            return;
        };
        self.lib_chosen = std::iter::once(hash).collect();
        self.lib_anchor = Some(index);
        self.select_lib_entry(index);
        self.hearing.arrows = Arrows::Tones;
    }

    /// Step through TonePush's tones as the Cloud draws them, and play each:
    /// each file is fetched once a session, and a fast run plays only the
    /// tone stopped on.
    pub(crate) fn step_cloud(&mut self, direction: i64, ctx: &egui::Context) {
        let order = self.cloud_rows.clone();
        if order.is_empty() || direction == 0 {
            return;
        }
        let at = self
            .cloud_selected
            .and_then(|selected| order.iter().position(|&row| row == selected));
        let next = match at {
            Some(at) => (at as i64 + direction).clamp(0, order.len() as i64 - 1) as usize,
            None if direction > 0 => 0,
            None => order.len() - 1,
        };
        if Some(next) == at {
            return;
        }
        self.cloud_selected = Some(order[next]);
        self.cloud_reveal_near = true;
        self.start_cloud_action(order[next], crate::CloudAction::Audition, ctx);
    }

    /// Space on TonePush's rows: play the chosen tone, or put back the one
    /// playing.
    pub(crate) fn space_on_cloud(&mut self, ctx: &egui::Context) {
        let Some(index) = self.cloud_selected else {
            return;
        };
        let Some(id) = self
            .cloud_entries
            .get(index)
            .map(|entry| entry.discovered.tone.summary.id)
        else {
            return;
        };
        if self.hears(&Source::TonePush(id)) {
            self.put_back();
        } else {
            self.start_cloud_action(index, crate::CloudAction::Audition, ctx);
        }
    }

    /// Space: play the chosen tone, or put back the one playing.
    pub(crate) fn space_on_tones(&mut self) {
        let Some(index) = self.lib_selected else {
            return;
        };
        let Some(hash) = self.lib_entries.get(index).map(|entry| entry.hash.clone()) else {
            return;
        };
        if self.hears(&Source::Library(hash)) {
            self.put_back();
        } else {
            self.audition_library(index);
        }
    }

    /// The worker says which tone the pedal plays now.
    pub(crate) fn audition_event(&mut self, key: Option<i64>) {
        let was = self.auditioning;
        self.auditioning = key;
        if key.is_none() {
            self.put_aside_dirty = false;
            // The audition that played has ended: by Put back or Keep, or by
            // the pedal moving on by itself. A tone asked for since, and not
            // played yet, is still on its way.
            let waiting = self
                .hearing
                .heard
                .as_ref()
                .is_some_and(|heard| Some(heard.key) != was);
            if !waiting {
                self.forget_heard();
            }
        }
    }

    /// The tone asked to play under this key could not be played.
    pub(crate) fn audition_failed(&mut self, key: i64) {
        if self
            .hearing
            .heard
            .as_ref()
            .is_some_and(|heard| heard.key == key)
        {
            self.hearing.heard = None;
            if self.auditioning.is_none() {
                self.hearing.set_aside = None;
            }
        }
    }

    /// The audition's keys, ahead of the pedal's own: Enter keeps in the
    /// loaded slot, Ctrl Enter in another, Esc puts back and Ctrl S points
    /// at the bar; on the library's tones the arrows step and play, and
    /// Space plays or puts back. Not while a field has the keyboard, a menu
    /// is open or a question is asked.
    pub(crate) fn audition_keys(&mut self, ctx: &egui::Context, tier: Tier) {
        if ctx.memory(|memory| memory.focused().is_some()) || ctx.any_popup_open() || self.asking()
        {
            return;
        }
        use egui::{Key, Modifiers};
        let consume = |modifiers: Modifiers, key: Key| {
            ctx.input_mut(|input| input.consume_key(modifiers, key))
        };
        if self.hearing() {
            if consume(Modifiers::COMMAND, Key::S) {
                self.hearing.nudged = Some(ctx.input(|input| input.time));
            }
            if self.sending.is_none() {
                if consume(Modifiers::COMMAND, Key::Enter) {
                    self.keep_heard_elsewhere();
                } else if consume(Modifiers::NONE, Key::Enter) {
                    self.keep_heard();
                } else if consume(Modifiers::NONE, Key::Escape) {
                    self.put_back();
                }
            }
        } else if self.hearing.glimpse.is_some() && consume(Modifiers::NONE, Key::Escape) {
            self.close_glimpse();
        } else if self.hearing.strip.is_some() && consume(Modifiers::NONE, Key::Escape) {
            self.hearing.strip = None;
        }
        let feed = self.hearing.arrows == Arrows::Cloud
            && self.lib_showing == LibraryView::Cloud
            && !self.library_folded(tier)
            && self.sending.is_none();
        if feed {
            if consume(Modifiers::NONE, Key::ArrowDown) {
                self.step_cloud(1, ctx);
            } else if consume(Modifiers::NONE, Key::ArrowUp) {
                self.step_cloud(-1, ctx);
            } else if consume(Modifiers::NONE, Key::Space) {
                self.space_on_cloud(ctx);
            }
        }
        let tones = self.hearing.arrows == Arrows::Tones
            && self.lib_showing == LibraryView::Tones
            && !self.library_folded(tier)
            && self.sending.is_none();
        if tones {
            if consume(Modifiers::NONE, Key::ArrowDown) {
                self.step_tones(1);
            } else if consume(Modifiers::NONE, Key::ArrowUp) {
                self.step_tones(-1);
            } else if consume(Modifiers::NONE, Key::Space) {
                self.space_on_tones();
            }
        }
    }

    /// Whether a question is open that the keys belong to.
    fn asking(&self) -> bool {
        self.confirm_switch.is_some()
            || self.confirm_clear.is_some()
            || self.confirm_push.is_some()
            || self.confirm_delete.is_some()
            || self.name_clash.is_some()
            || self.put_question.is_some()
    }

    /// The bar on the library pane's top edge: what plays and what waits,
    /// the keys that step, Put back and Keep. In its place, the strip that
    /// says why a click did not play.
    pub(crate) fn audition_bar(&mut self, ui: &mut Ui, rect: Rect, tier: Tier) {
        let Some(heard) = self.hearing.heard.clone() else {
            if let Some(strip) = self.hearing.strip.clone() {
                self.strip_bar(ui, rect, &strip);
            }
            return;
        };
        let loading = self.heard_loading();
        let set_aside = self.hearing.set_aside.clone().unwrap_or_default();
        ui.painter().rect_filled(
            rect,
            CornerRadius::ZERO,
            theme::mix(theme::bg(), theme::accent(), 0.07),
        );
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, theme::accent_line()),
        );
        let inner = Rect::from_min_max(
            Pos2::new(rect.left() + 16.0, rect.top()),
            Pos2::new(rect.right() - 10.0, rect.bottom()),
        );
        let small = tier == Tier::S;
        let mut keep = false;
        let mut elsewhere = false;
        let mut library = false;
        let mut back = false;
        let mut unlock = false;
        let mut send_back = None;
        let mut step = 0;
        let nudge = self.hearing.nudged.and_then(|at| {
            let since = ui.input(|input| input.time) - at;
            (since < NUDGE).then_some((since / NUDGE) as f32)
        });
        if nudge.is_none() {
            self.hearing.nudged = None;
        }
        let actions = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .id_salt("audition-bar-actions")
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    let label = if set_aside.slot.is_empty() {
                        "Keep".to_owned()
                    } else {
                        format!("Keep in {}", set_aside.slot)
                    };
                    let (action, chevron) = theme::split_button(
                        ui,
                        egui::Id::new("audition-keep"),
                        &label,
                        (!small).then_some("Enter"),
                        !loading,
                    );
                    if let Some(t) = nudge {
                        // Ctrl S points here: what plays is saved once kept.
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(16));
                        let ring = action.rect.union(chevron.rect).expand(2.0 + 5.0 * t);
                        ui.painter().rect_stroke(
                            ring,
                            CornerRadius::same(9),
                            Stroke::new(2.0, theme::alpha(theme::accent(), 1.0 - t)),
                            egui::StrokeKind::Outside,
                        );
                    }
                    keep = action
                        .on_hover_text(format!(
                            "Make it {}'s edit: Save writes it, Undo brings back {}",
                            set_aside.slot, set_aside.name
                        ))
                        .clicked();
                    egui::Popup::menu(&chevron)
                        .align(egui::RectAlign::BOTTOM_END)
                        .gap(6.0)
                        .show(|ui| {
                            theme::menu_width(ui, 300.0);
                            let read_only = self
                                .pro_active()
                                .then(|| self.pro.read_only_firmware())
                                .flatten();
                            let aside = read_only.as_ref().map_or_else(
                                || heard.source.words().to_owned(),
                                |firmware| format!("on firmware {firmware}"),
                            );
                            theme::menu_header(
                                ui,
                                &format!("Keep {}", heard.name),
                                Some(aside.as_str()),
                            );
                            let here = format!("In {}, as an edit", set_aside.slot);
                            keep |= theme::menu_keyed(
                                ui,
                                Some(Icon::Pedal),
                                &here,
                                &["Enter"],
                                !loading,
                            )
                            .clicked();
                            match self.elsewhere_refusal() {
                                Some(why) => {
                                    theme::menu_disabled(
                                        ui,
                                        Some(Icon::ArrowDownToLine),
                                        "In another slot…",
                                        Some(why.as_str()),
                                    );
                                }
                                None => {
                                    elsewhere |= theme::menu_keyed(
                                        ui,
                                        Some(Icon::ArrowDownToLine),
                                        "In another slot…",
                                        &["Ctrl", "Enter"],
                                        !loading,
                                    )
                                    .clicked();
                                }
                            }
                            if self.pro_active() && self.pro.unlock_offer().is_some() {
                                unlock |= theme::menu_item(
                                    ui,
                                    Some(Icon::ShieldCheck),
                                    "Back up to unlock saving",
                                    None,
                                )
                                .clicked();
                            }
                            if matches!(heard.source, Source::TonePush(_)) {
                                library |= theme::menu_keyed(
                                    ui,
                                    Some(Icon::CloudDownload),
                                    "In your library",
                                    &["Ctrl", "D"],
                                    !loading,
                                )
                                .clicked();
                            }
                            theme::menu_separator(ui);
                            let note = match &read_only {
                                Some(firmware) => format!(
                                    "TonePush plays and changes presets on {firmware} but does \
                                     not write them yet. Kept in {slot}, {tone} plays until you \
                                     load another preset; Save, renaming and writing a slot wait \
                                     for firmware TonePush has verified.",
                                    slot = set_aside.slot,
                                    tone = heard.name,
                                ),
                                None => format!(
                                    "In {slot}, Save then writes it over {name}, and Undo still \
                                     brings {name} back. Another slot is written once you confirm \
                                     which, and {name} comes back with its changes.",
                                    slot = set_aside.slot,
                                    name = set_aside.name,
                                ),
                            };
                            theme::menu_note(ui, &note);
                        });
                    if let Source::Setlist { slot, hash, .. } = &heard.source {
                        let label = format!("Send to {}", self.active_slot_label(*slot));
                        let refused = self.put_refusal();
                        let sent = theme::Button::new(&label)
                            .icon(Icon::Download)
                            .enabled(!loading && refused.is_none())
                            .show(ui)
                            .on_hover_text(
                                "Write the setlist's version back into its slot, asking first",
                            )
                            .on_disabled_hover_text(refused.unwrap_or_default());
                        if sent.clicked() {
                            send_back = Some((*slot, hash.clone()));
                        }
                    }
                    if matches!(heard.source, Source::TonePush(_)) && !small {
                        library |= theme::Button::new("Keep in library")
                            .icon(Icon::CloudDownload)
                            .hint("Ctrl D")
                            .enabled(!loading)
                            .show(ui)
                            .on_hover_text("Keep it in your library, leaving the pedal as it is")
                            .clicked();
                    }
                    let back_label = if small || set_aside.name.is_empty() {
                        "Put back".to_owned()
                    } else {
                        format!("Put {} back", set_aside.name)
                    };
                    back = theme::Button::new(&back_label)
                        .hint("Esc")
                        .show(ui)
                        .on_hover_text("Write the preset back exactly as it was, changes and all")
                        .clicked();
                    if !small && self.hearing.arrows != Arrows::Presets {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        theme::label(
                            ui,
                            if tier == Tier::L {
                                "try the next"
                            } else {
                                "next"
                            },
                            theme::regular(12.0),
                            theme::muted(),
                        );
                        for (key, direction) in [("↓", 1), ("↑", -1)] {
                            if theme::keycaps(ui, &[key])
                                .interact(Sense::click())
                                .on_hover_text(if direction > 0 {
                                    "Play the next tone"
                                } else {
                                    "Play the tone before"
                                })
                                .clicked()
                            {
                                step = direction;
                            }
                        }
                    }
                },
            )
            .response
            .rect;
        // The chip and the words, in what is left.
        let words_rect = Rect::from_min_max(
            inner.min,
            Pos2::new((actions.left() - 12.0).max(inner.left()), inner.bottom()),
        );
        let mut words_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(words_rect)
                .id_salt("audition-bar-words")
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        words_ui.set_clip_rect(words_rect.intersect(ui.clip_rect()));
        words_ui.spacing_mut().item_spacing.x = 12.0;
        let chip = if loading {
            theme::Chip::new("Loading").spinner()
        } else {
            theme::Chip::new("Playing").icon(Icon::Volume)
        };
        chip.mood(theme::Mood::Accent)
            .outline(theme::accent_line())
            .height(22.0)
            .show(&mut words_ui);
        let slot = set_aside.slot.clone();
        // Edits made while it plays stay with it, and the bar says so.
        let edits = if loading {
            0
        } else if self.pro_active() {
            self.pro.undo_depth()
        } else {
            self.undo_depth
        };
        let with = match edits {
            0 => String::new(),
            1 => ", with 1 change".to_owned(),
            n => format!(", with {n} changes"),
        };
        let mut parts: Vec<(String, bool)> = vec![(heard.name.clone(), true)];
        if let Source::Setlist { slot: from, .. } = &heard.source {
            parts.push((
                format!(", the setlist's {},", self.active_slot_label(*from)),
                false,
            ));
        }
        if small {
            parts.push((format!("{with} in {slot}"), false));
            if !set_aside.name.is_empty() {
                parts.push((format!(" · {} set aside", set_aside.name), false));
            }
        } else {
            parts.push((
                if loading {
                    format!(" is on its way to {slot}. ")
                } else {
                    format!(" is playing in {slot}{with}. ")
                },
                false,
            ));
            if !set_aside.name.is_empty() {
                parts.push((set_aside.name.clone(), true));
                parts.push((
                    if set_aside.dirty {
                        " waits, with its unsaved changes.".to_owned()
                    } else {
                        " waits.".to_owned()
                    },
                    false,
                ));
            }
        }
        let room = words_ui.available_width();
        paint_words(&mut words_ui, &parts, room);

        if step != 0 {
            self.step_tones(step);
        }
        if unlock {
            self.pro.back_up_here();
        }
        if let Some((slot, hash)) = send_back {
            self.put_to(slot, vec![(hash, heard.name.clone())]);
        }
        if keep {
            self.keep_heard();
        } else if elsewhere {
            self.keep_heard_elsewhere();
        } else if library {
            if let Source::TonePush(key) = heard.source {
                self.keep_cloud_heard(key);
            }
        } else if back {
            self.put_back();
        }
    }

    /// The strip where the bar goes, in the info voice: why a click did not
    /// play, and what would make it.
    fn strip_bar(&mut self, ui: &mut Ui, rect: Rect, strip: &Strip) {
        ui.painter().rect_filled(
            rect,
            CornerRadius::ZERO,
            theme::mix(theme::bg(), theme::info(), 0.07),
        );
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, theme::alpha(theme::info(), 0.4)),
        );
        let inner = Rect::from_min_max(
            Pos2::new(rect.left() + 16.0, rect.top()),
            Pos2::new(rect.right() - 10.0, rect.bottom()),
        );
        let mut narrow = false;
        let actions = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .id_salt("audition-strip-actions")
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| {
                    if let Some(url) = &strip.open {
                        if theme::Button::new("Open")
                            .ghost()
                            .icon(Icon::ExternalLink)
                            .show(ui)
                            .clicked()
                        {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url.clone()));
                        }
                    }
                    if let Some(pedal) = &strip.narrow {
                        narrow = theme::Button::new(&format!("Show only what the {pedal} plays"))
                            .ghost()
                            .icon(Icon::Filter)
                            .show(ui)
                            .clicked();
                    }
                },
            )
            .response
            .rect;
        let words_rect = Rect::from_min_max(
            inner.min,
            Pos2::new((actions.left() - 12.0).max(inner.left()), inner.bottom()),
        );
        let mut words_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(words_rect)
                .id_salt("audition-strip-words")
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        words_ui.set_clip_rect(words_rect.intersect(ui.clip_rect()));
        words_ui.spacing_mut().item_spacing.x = 12.0;
        let (spot, _) = words_ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        theme::paint_icon(&words_ui, strip.icon, spot.center(), 16.0, theme::info());
        let room = words_ui.available_width();
        paint_words(&mut words_ui, &strip.words, room);
        if narrow {
            self.library_device_filter = strip.narrow.clone();
            self.hearing.strip = None;
            self.lib_selected = None;
            self.cloud_loaded_device = None;
        }
    }
}

impl App {
    /// A tone shown, not played, with no pedal: its name and where it came
    /// from in the deck, its chain on the board and the chosen block's face,
    /// all read only, with the way back to the connect page.
    pub(crate) fn glimpse_page(&mut self, root: &mut Ui, tier: Tier) {
        let Some(mut glimpse) = self.hearing.glimpse.take() else {
            return;
        };
        let mut back = false;
        egui::Panel::top("glimpse-deck")
            .exact_size(shell::deck_height(tier))
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::bg()))
            .show(root, |ui| {
                let full = ui.max_rect();
                let right = ui
                    .scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(full.with_max_x(full.right() - 16.0))
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        |ui| {
                            back = theme::Button::new("Back to Plug in your pedal")
                                .ghost()
                                .icon(Icon::Usb)
                                .hint("Esc")
                                .show(ui)
                                .clicked();
                        },
                    )
                    .response
                    .rect;
                let left = full.left() + if self.sidebar_hidden { 12.0 } else { 20.0 };
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_max(
                            Pos2::new(left, full.top()),
                            Pos2::new(right.left() - 14.0, full.bottom()),
                        ))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 14.0;
                        theme::Chip::new("Preview")
                            .icon(Icon::Eye)
                            .height(24.0)
                            .show(ui);
                        let width = ui.available_width();
                        ui.allocate_ui_with_layout(
                            Vec2::new(width, 44.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                let size = tier.pick(theme::PRESET_NAME, theme::PRESET_NAME, 22.0);
                                let title = shell::title_galley(
                                    ui,
                                    &glimpse.preview.name,
                                    size,
                                    theme::text(),
                                    width,
                                );
                                let (place, _) = ui.allocate_exact_size(
                                    Vec2::new(width, size + 4.0),
                                    Sense::hover(),
                                );
                                shell::paint_line(ui, title, place.left(), place.center().y);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    theme::label(
                                        ui,
                                        &glimpse.from,
                                        theme::regular(12.0),
                                        theme::muted(),
                                    );
                                    theme::label(ui, "·", theme::regular(12.0), theme::faint());
                                    crate::library_view::marker(ui, glimpse.marker.as_ref(), true);
                                    theme::label(ui, "·", theme::regular(12.0), theme::faint());
                                    theme::label(
                                        ui,
                                        "Not playing: no pedal is connected",
                                        theme::regular(12.0),
                                        theme::muted(),
                                    );
                                });
                            },
                        );
                    },
                );
            });

        // The one renderer draws whatever chain the app holds, so for this
        // page it holds the tone's, read only.
        std::mem::swap(&mut self.chain, &mut glimpse.preview.chain);
        std::mem::swap(&mut self.layout, &mut glimpse.preview.layout);
        let selected = self.selected;
        self.selected = self
            .chain
            .iter()
            .position(|block| self.block_category(block).as_deref() == Some("Amp"))
            .or_else(|| self.chain.iter().position(|block| self.is_effect(block)))
            .unwrap_or(0);
        self.display_only = true;
        self.hearing.glimpsing = true;
        self.signal_chain(root);
        self.library_pane(root, tier);
        let rect = root.available_rect_before_wrap();
        let mut face = root.new_child(egui::UiBuilder::new().max_rect(rect));
        face.disable();
        self.pane(&mut face, tier);
        self.hearing.glimpsing = false;
        self.display_only = false;
        self.selected = selected;
        std::mem::swap(&mut self.chain, &mut glimpse.preview.chain);
        std::mem::swap(&mut self.layout, &mut glimpse.preview.layout);
        // A click in the library may have shown another tone meanwhile.
        if self.hearing.glimpse.is_none() && !back {
            self.hearing.glimpse = Some(glimpse);
        }
        if back {
            self.close_glimpse();
        }
    }
}

/// The menu Keep's chevron opens. The screenshots open it.
#[cfg(test)]
pub(crate) fn keep_menu() -> egui::Id {
    egui::Id::new("audition-keep").with("menu").with("popup")
}

/// A line of plain and bold words, cut to `width` with an ellipsis.
fn paint_words(ui: &mut Ui, parts: &[(String, bool)], width: f32) {
    let mut job = egui::text::LayoutJob::default();
    for (text, bold) in parts {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: if *bold {
                    theme::semibold(12.5)
                } else {
                    theme::regular(12.5)
                },
                color: if *bold {
                    theme::text()
                } else {
                    theme::text_soft()
                },
                ..Default::default()
            },
        );
    }
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(20.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.painter().layout_job(job);
    let (place, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(place.min, galley, Color32::PLACEHOLDER);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::Marker;
    use crate::session::Evt;
    use crate::Connection;
    use std::sync::mpsc;

    fn app() -> (App, mpsc::Sender<Evt>, mpsc::Receiver<Cmd>) {
        let (to_device, cmds) = mpsc::channel();
        let (events, from_device) = mpsc::channel();
        let app = App::new(&egui::Context::default(), to_device, from_device);
        // It reaches for the pedal as it starts.
        let _ = cmds.try_iter().count();
        (app, events, cmds)
    }

    /// An HX Stomp with 01B Plexi Crunch loaded, edited and not saved.
    fn online(app: &mut App) {
        app.connection = Connection::Online;
        app.device = "HX Stomp".into();
        app.firmware = "3.80".into();
        app.library_connected_device = "HX Stomp".into();
        app.preset_index = 1;
        app.preset_name = "Plexi Crunch".into();
        app.dirty = true;
    }

    /// A tone kept in the scratch library, made for `marker`.
    fn tone(name: &str, marker: Marker) -> LibEntry {
        let kind = if marker.family == crate::devices::Family::Pro {
            "vxpreset"
        } else {
            "hxpreset"
        };
        let hash = library::store(name, name.as_bytes(), kind).expect("the scratch library");
        LibEntry {
            hash: hash.clone(),
            series: hash,
            name: name.to_owned(),
            line: String::new(),
            meta: library::Meta::default(),
            added_at: String::new(),
            modified_at: String::new(),
            downloads: None,
            rating: None,
            version: 1,
            versions: 1,
            chain: Vec::new(),
            pro: kind == "vxpreset",
            marker: Some(marker),
            firmware: "3.80".to_owned(),
        }
    }

    fn played(cmds: &mpsc::Receiver<Cmd>) -> (i64, String) {
        match cmds.try_recv() {
            Ok(Cmd::AuditionDocument { key, name, .. }) => (key, name),
            Ok(_) => panic!("expected an audition, got another command"),
            Err(_) => panic!("expected an audition, got nothing"),
        }
    }

    /// One click plays the tone in the loaded preset's place, setting that
    /// preset aside with its unsaved changes; Keep makes it the slot's edit.
    #[test]
    fn a_click_plays_a_library_tone_and_keep_makes_it_the_edit() {
        let _scratch = library::tests::Scratch::new("audition-click");
        let (mut app, events, cmds) = app();
        online(&mut app);
        app.lib_entries = vec![tone("Dream Pop", Marker::hx("Stomp"))];

        app.audition_library(0);
        let (key, name) = played(&cmds);
        assert_eq!(name, "Dream Pop");
        assert!(app.heard_loading(), "on its way until the pedal says");
        let set_aside = app.hearing.set_aside.clone().expect("set aside");
        assert_eq!(
            (
                set_aside.slot.as_str(),
                set_aside.name.as_str(),
                set_aside.dirty
            ),
            ("01B", "Plexi Crunch", true)
        );
        assert!(app.bar_shown());

        events.send(Evt::Auditioning(Some(key))).unwrap();
        app.drain_events();
        assert!(app.heard_playing().is_some());
        let shown = app.shown_hearing().expect("shown");
        assert_eq!(shown.board_note(), "Auditioning Dream Pop");

        app.audition_library(0);
        assert!(
            cmds.try_recv().is_err(),
            "a click on what plays plays nothing new"
        );

        app.keep_heard();
        assert!(matches!(cmds.try_recv(), Ok(Cmd::KeepAudition)));
        assert!(!app.bar_shown());
    }

    /// The arrows step through the tones as drawn, each one played against
    /// the same baseline: what the first audition set aside.
    #[test]
    fn stepping_plays_each_tone_against_the_first_baseline() {
        let _scratch = library::tests::Scratch::new("audition-step");
        let (mut app, events, cmds) = app();
        online(&mut app);
        app.lib_entries = vec![
            tone("Brown Lead", Marker::hx("Stomp")),
            tone("Doom Fuzz", Marker::hx("Stomp")),
            tone("Dream Pop", Marker::hx("Stomp")),
        ];
        app.lib_order = vec![2, 0, 1];
        app.choose_tone(2);

        app.step_tones(1);
        let (first, name) = played(&cmds);
        assert_eq!(name, "Brown Lead");
        assert_eq!(app.lib_selected, Some(0));
        events.send(Evt::Auditioning(Some(first))).unwrap();
        app.drain_events();
        // The deck now names what plays; the baseline stays Plexi Crunch.
        app.preset_name = "Brown Lead".into();
        app.dirty = false;

        app.step_tones(1);
        let (_, name) = played(&cmds);
        assert_eq!(name, "Doom Fuzz");
        app.step_tones(1);
        assert!(
            cmds.try_recv().is_err(),
            "the last row is as far as it goes"
        );
        let set_aside = app.hearing.set_aside.clone().expect("set aside");
        assert_eq!(set_aside.name, "Plexi Crunch");
        assert!(set_aside.dirty);

        app.put_back();
        assert!(matches!(cmds.try_recv(), Ok(Cmd::EndAudition)));
        assert!(!app.bar_shown());
    }

    /// The end of one audition reported after another tone was asked for
    /// does not forget the one on its way.
    #[test]
    fn an_end_on_its_way_does_not_forget_the_tone_asked_since() {
        let _scratch = library::tests::Scratch::new("audition-order");
        let (mut app, events, cmds) = app();
        online(&mut app);
        app.lib_entries = vec![
            tone("Brown Lead", Marker::hx("Stomp")),
            tone("Doom Fuzz", Marker::hx("Stomp")),
        ];
        app.audition_library(0);
        let (first, _) = played(&cmds);
        events.send(Evt::Auditioning(Some(first))).unwrap();
        app.drain_events();

        app.put_back();
        let _ = cmds.try_iter().count();
        app.audition_library(1);
        let (second, _) = played(&cmds);
        events.send(Evt::Auditioning(None)).unwrap();
        app.drain_events();
        assert!(app.heard_loading(), "Doom Fuzz is still on its way");
        events.send(Evt::Auditioning(Some(second))).unwrap();
        app.drain_events();
        assert_eq!(
            app.heard_playing().map(|heard| heard.name.as_str()),
            Some("Doom Fuzz")
        );

        // The pedal moving on by itself ends it, with nothing left to show.
        events.send(Evt::Auditioning(None)).unwrap();
        app.drain_events();
        assert!(!app.bar_shown());
        assert!(!app.hearing());
    }

    /// A tone the pedal cannot play says why where the bar goes, offers to
    /// show only what the pedal plays, and sends nothing.
    #[test]
    fn a_tone_the_pedal_cannot_play_says_why() {
        let _scratch = library::tests::Scratch::new("audition-refused");
        let (mut app, _events, cmds) = app();
        online(&mut app);
        app.library_device_filter = None;
        app.lib_entries = vec![tone("Velvet Drive", Marker::pro("2.x"))];

        app.audition_library(0);
        assert!(cmds.try_recv().is_err());
        assert!(!app.hearing());
        let strip = app.hearing.strip.clone().expect("a reason");
        let words: String = strip.words.iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(
            words,
            "Velvet Drive is a StompStation PRO tone. The HX Stomp cannot play it, so it \
             stays on Plexi Crunch."
        );
        assert_eq!(strip.words[0], ("Velvet Drive".to_owned(), true));
        assert_eq!(strip.narrow.as_deref(), Some("HX Stomp"));
    }

    /// A setlist's slot plays the setlist's version, says where it came
    /// from, and Send writes it back into its slot after asking.
    #[test]
    fn a_setlist_slot_plays_and_sends_back_to_its_slot() {
        let _scratch = library::tests::Scratch::new("audition-setlist");
        let (mut app, events, cmds) = app();
        online(&mut app);
        app.presets = (0..16).map(|index| format!("Preset {index}")).collect();
        let tone = tone("Shimmer Pad", Marker::hx("Stomp"));
        let slot = library::Slot {
            hash: tone.hash.clone(),
            name: tone.name.clone(),
            file: String::new(),
        };
        app.audition_setlist_slot("Album release show", 12, &slot);
        let (key, name) = played(&cmds);
        assert_eq!(name, "Shimmer Pad");
        events.send(Evt::Auditioning(Some(key))).unwrap();
        app.drain_events();
        let shown = app.shown_hearing().expect("shown");
        assert_eq!(shown.from, "from Album release show");
        assert_eq!(shown.aside.as_deref(), Some("its 05A"));
        assert_eq!(
            shown.board_note(),
            "Auditioning Shimmer Pad, from Album release show"
        );

        app.put_to(12, vec![(slot.hash.clone(), slot.name.clone())]);
        let asking = app
            .put_question
            .clone()
            .expect("05A holds a preset: it asks");
        assert_eq!(asking.writes[0].replaces.as_deref(), Some("Preset 12"));
    }

    /// An older version plays under its own number, against the same
    /// baseline as any other tone.
    #[test]
    fn an_older_version_plays_under_its_number() {
        let _scratch = library::tests::Scratch::new("audition-version");
        let (mut app, _events, cmds) = app();
        online(&mut app);
        app.lib_entries = vec![tone("Plexi Crunch", Marker::hx("Stomp"))];
        let older = library::store("Plexi Crunch", b"as it was", "hxpreset").unwrap();
        app.audition_version(0, older.clone(), 1);
        let (_, name) = played(&cmds);
        assert_eq!(name, "Plexi Crunch v1");
        assert!(app.hears(&Source::Library(older)));
    }

    /// With no pedal, a click plays nothing and says what would make it
    /// play; the way back to the connect page is Esc.
    #[test]
    fn with_no_pedal_a_click_says_why_and_plays_nothing() {
        let _scratch = library::tests::Scratch::new("audition-offline");
        let (mut app, _events, cmds) = app();
        app.connection = Connection::Offline;
        app.lib_entries = vec![tone("Dream Pop", Marker::hx("Stomp"))];
        app.audition_library(0);
        assert!(cmds.try_recv().is_err());
        assert!(!app.hearing());
        let strip = app.hearing.strip.clone().expect("a reason");
        let words: String = strip.words.iter().map(|(text, _)| text.as_str()).collect();
        assert!(
            words.starts_with("No pedal is connected, so Dream Pop is"),
            "{words}"
        );
        assert!(words.ends_with("Plug in your HX Stomp and the next click plays."));
        app.close_glimpse();
        assert!(app.hearing.strip.is_none());
    }

    /// A TonePush tone hosted by another catalog is not opened on a click:
    /// the strip says where it lives and offers the way there.
    #[test]
    fn a_tone_hosted_elsewhere_says_so_with_the_way_to_it() {
        let (mut app, _events, _cmds) = app();
        online(&mut app);
        let mut tone: crate::cloud::ToneDetails =
            serde_json::from_str(include_str!("../tests/fixtures/cloud/tone-details.json"))
                .unwrap();
        tone.download = Some(crate::cloud::ToneDownload {
            artifact: None,
            external: Some("https://line6.com/customtone/tone/123/".to_owned()),
        });
        let entry = crate::cloud::DiscoveredTone {
            song: tone.song.clone().unwrap(),
            tone,
        };
        let ctx = egui::Context::default();
        app.start_cloud_entry_action(entry, crate::CloudAction::Audition, &ctx);
        let strip = app.hearing.strip.clone().expect("a reason");
        assert_eq!(
            strip.words[0].0,
            "Hosted by Line 6 CustomTone: TonePush cannot play it from here."
        );
        assert_eq!(
            strip.open.as_deref(),
            Some("https://line6.com/customtone/tone/123/")
        );
        assert!(app.cloud_download.is_none(), "nothing is fetched");
    }

    /// The arrows step through TonePush's tones as drawn, each played from
    /// its file once it is here.
    #[test]
    fn stepping_through_tonepush_plays_each_tone() {
        let (mut app, _events, cmds) = app();
        online(&mut app);
        let tone: crate::cloud::ToneDetails =
            serde_json::from_str(include_str!("../tests/fixtures/cloud/tone-details.json"))
                .unwrap();
        let document = include_bytes!("../../hx-proto/tests/preset.bin").to_vec();
        for id in [456, 457] {
            let mut tone = tone.clone();
            tone.summary.id = id;
            tone.summary.name = format!("Tone {id}");
            let discovered = crate::cloud::DiscoveredTone {
                song: tone.song.clone().unwrap(),
                tone,
            };
            app.cloud_entries.push(crate::CloudEntry {
                row: App::cloud_row(&discovered),
                discovered,
            });
            app.cloud_artifacts.insert(id, document.clone());
        }
        app.cloud_rows = vec![1, 0];
        let ctx = egui::Context::default();
        app.step_cloud(1, &ctx);
        assert_eq!(app.cloud_selected, Some(1));
        match cmds.try_recv() {
            Ok(Cmd::AuditionDocument { key, name, .. }) => {
                assert_eq!((key, name.as_str()), (457, "Tone 457"));
            }
            _ => panic!("the first row plays"),
        }
        app.step_cloud(1, &ctx);
        assert_eq!(app.cloud_selected, Some(0));
        assert!(app.hears(&Source::TonePush(456)));
    }

    /// A tone that fails to play takes the bar with it, and nothing it set
    /// aside is remembered.
    #[test]
    fn a_tone_that_fails_to_play_leaves_nothing_behind() {
        let _scratch = library::tests::Scratch::new("audition-failed");
        let (mut app, events, cmds) = app();
        online(&mut app);
        app.lib_entries = vec![tone("Dream Pop", Marker::hx("Stomp"))];
        app.audition_library(0);
        let (key, _) = played(&cmds);
        events.send(Evt::AuditionFailed(key)).unwrap();
        app.drain_events();
        assert!(!app.bar_shown());
        assert!(app.hearing.set_aside.is_none());
    }
}
