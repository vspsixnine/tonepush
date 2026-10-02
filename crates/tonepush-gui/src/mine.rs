//! The Cloud's Mine (docs/design/library-workflow-2026-10-02, 18 and 19): what
//! you published on TonePush. Where the server lists an account's tones and
//! setlists (`account.rs`), it lists everything you published, from any
//! computer, with who can see each one. Where it does not, it lists what this
//! library published, from the records it keeps when it publishes, with what
//! TonePush answers for each one: downloads, its versions and the one people
//! get, and whether the library has moved on since. Tones published before
//! those records were kept are found once, by name and the exact file hash.

use std::collections::BTreeMap;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use egui::{Pos2, Rect, Stroke, Ui, Vec2};

use crate::audition::Arrows;
use crate::cloud;
use crate::library;
use crate::library_view::{marker, section, version_row, where_row, Words};
use crate::table;
use crate::theme::{self, Icon, Tier};
use crate::{App, CloudAction};

/// How long TonePush's answers are kept before Mine asks again.
const FRESH_FOR: Duration = Duration::from_secs(120);

/// How many tones one search for earlier publishes looks for.
const BACKFILL_AT_MOST: usize = 25;

/// The columns of Mine's table, in order.
const PLACES: usize = 0;
const PEDAL: usize = 1;
const NAME: usize = 2;
const SONG: usize = 3;
const ON_TONEPUSH: usize = 4;
const DOWNLOADS: usize = 5;
const PUBLISHED: usize = 6;

/// TonePush's answers for published tones, by id: each tone, or why it could
/// not be read.
type Answers = Vec<(i64, Result<cloud::ToneDetails, String>)>;

/// What Mine holds between frames.
pub(crate) struct Mine {
    /// TonePush's answer for each published tone, by its id: the tone, or
    /// why it could not be read.
    pub details: BTreeMap<i64, Result<cloud::ToneDetails, String>>,
    fetching: Option<Receiver<Answers>>,
    fetched: Option<Instant>,
    /// The tone chosen, by its id on TonePush.
    pub selected: Option<i64>,
    /// The setlist chosen instead, by its id on TonePush.
    pub selected_setlist: Option<i64>,
    /// The rows as the table draws them, for the arrows.
    pub order: Vec<i64>,
    /// Which column orders the rows, and which way.
    pub sort: (usize, bool),
    /// The search for tones published before records were kept.
    backfill: Option<Receiver<Vec<(String, library::Published)>>>,
    backfilled: bool,
    /// Scroll the chosen row into view, near where it is.
    pub reveal: bool,
}

impl Default for Mine {
    fn default() -> Self {
        Mine {
            details: BTreeMap::new(),
            fetching: None,
            fetched: None,
            selected: None,
            selected_setlist: None,
            order: Vec::new(),
            sort: (DOWNLOADS, false),
            backfill: None,
            backfilled: false,
            reveal: false,
        }
    }
}

impl Mine {
    /// Whether TonePush has been asked this session.
    pub(crate) fn asked(&self) -> bool {
        self.fetched.is_some()
    }

    /// Hold what is known as fresh, and the search for earlier publishes as
    /// done: a screenshot's answers are its own, and nothing is asked.
    #[cfg(test)]
    pub(crate) fn mark_asked(&mut self) {
        self.fetched = Some(Instant::now());
        self.backfilled = true;
    }
}

/// One published tone, as the library recorded it and TonePush answered.
#[derive(Clone)]
pub(crate) struct MineRow {
    pub record: library::Published,
    pub details: Option<cloud::ToneDetails>,
    /// Why TonePush's answer is missing, when it is.
    pub problem: Option<String>,
    /// The library's row for this tone, when it still holds it.
    pub local: Option<usize>,
}

impl MineRow {
    pub(crate) fn name(&self) -> String {
        self.details
            .as_ref()
            .map(|details| details.summary.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| self.record.name.clone())
    }

    /// The version people get, and how many there are.
    pub(crate) fn versions(&self) -> Option<(u32, u32)> {
        let details = self.details.as_ref()?;
        let current = details
            .summary
            .version_number
            .or_else(|| {
                details
                    .versions
                    .iter()
                    .find(|v| v.current)
                    .map(|v| v.number)
            })
            .unwrap_or(1);
        Some((current, details.summary.versions_count.max(current)))
    }

    pub(crate) fn downloads(&self) -> u64 {
        self.details.as_ref().map_or(0, |details| {
            details
                .summary
                .downloads_count
                .unwrap_or(details.summary.installs_count)
        })
    }

    /// When it first went on TonePush.
    pub(crate) fn published_at(&self) -> String {
        self.details
            .as_ref()
            .map(|details| details.summary.created_at.clone())
            .filter(|at| !at.is_empty())
            .unwrap_or_else(|| self.record.at.clone())
    }

    /// The tone as the Cloud plays it.
    pub(crate) fn discovered(&self) -> Option<cloud::DiscoveredTone> {
        let tone = self.details.clone()?;
        let song = tone.song.clone().unwrap_or_else(|| cloud::SongSummary {
            id: tone.summary.song_id,
            title: tone.summary.name.clone(),
            kind: cloud::SongKind::Original,
            artist: None,
            part: None,
            description: None,
            tags: Vec::new(),
            genres: Vec::new(),
            tuning: None,
            guitar_type: None,
            pickup_type: None,
            pickup_electronics: None,
            tone_count: 1,
            devices: Vec::new(),
            file_sha256s: Vec::new(),
        });
        Some(cloud::DiscoveredTone { song, tone })
    }
}

/// What publishing again would give TonePush that it does not have: the
/// library's revision, when it is not the file people get.
pub(crate) struct Ahead {
    pub hash: String,
    pub version: u32,
}

impl App {
    /// Every tone you published, as rows: the account's, where the server
    /// lists them, else this library's.
    pub(crate) fn mine_rows(&self) -> Vec<MineRow> {
        if let Some(tones) = &self.account.tones {
            return tones
                .iter()
                .map(|tone| {
                    let id = crate::publish::stable_id(&tone.summary);
                    let recorded = self
                        .published
                        .iter()
                        .find(|(_, record)| record.tone_id == id);
                    let series = recorded
                        .map(|(series, _)| series.clone())
                        .or_else(|| tone.summary.series_id.clone());
                    MineRow {
                        record: recorded.map_or_else(
                            || library::Published {
                                tone_id: id,
                                song_id: tone.summary.song_id,
                                hash: String::new(),
                                name: tone.summary.name.clone(),
                                at: tone.summary.created_at.clone(),
                            },
                            |(_, record)| record.clone(),
                        ),
                        details: Some(tone.clone()),
                        problem: None,
                        local: series.and_then(|series| {
                            self.lib_entries
                                .iter()
                                .position(|entry| entry.series == series)
                        }),
                    }
                })
                .collect();
        }
        self.published
            .iter()
            .map(|(series, record)| {
                let answer = self.mine.details.get(&record.tone_id);
                MineRow {
                    record: record.clone(),
                    details: answer.and_then(|answer| answer.as_ref().ok()).cloned(),
                    problem: answer.and_then(|answer| answer.as_ref().err()).cloned(),
                    local: self
                        .lib_entries
                        .iter()
                        .position(|entry| &entry.series == series),
                }
            })
            .collect()
    }

    /// How many things you published, for the Cloud's Mine.
    pub(crate) fn mine_count(&self) -> usize {
        match &self.account.tones {
            Some(tones) => tones.len() + self.account.setlists.as_ref().map_or(0, Vec::len),
            None => self.published.len(),
        }
    }

    /// Your setlists on TonePush, where the server keeps them.
    pub(crate) fn mine_setlists(&self) -> Vec<cloud::SetlistSummary> {
        self.account.setlists.clone().unwrap_or_default()
    }

    /// The library's setlist of the same name, the newest version.
    pub(crate) fn local_setlist_named(&self, name: &str) -> Option<usize> {
        self.lib_setlists
            .iter()
            .enumerate()
            .filter(|(_, (_, setlist))| setlist.name.eq_ignore_ascii_case(name))
            .max_by_key(|(_, (_, setlist))| setlist.revision())
            .map(|(index, _)| index)
    }

    /// The library's revision of a published tone when TonePush does not
    /// have it as the version people get.
    pub(crate) fn mine_ahead(&self, row: &MineRow) -> Option<Ahead> {
        let local = &self.lib_entries[row.local?];
        let current = row.details.as_ref()?.file_sha256.clone()?;
        let portable = self
            .portable_hashes
            .get(&local.hash)
            .cloned()
            .or_else(|| library::portable_hash(&local.hash))?;
        (portable != current).then(|| Ahead {
            hash: local.hash.clone(),
            version: local.version,
        })
    }

    /// The library's revision whose file is this version on TonePush.
    pub(crate) fn mine_local_version(&self, row: &MineRow, file: &str) -> Option<String> {
        let local = &self.lib_entries[row.local?];
        library::versions_of(&local.hash)
            .into_iter()
            .map(|version| version.hash)
            .find(|hash| {
                self.portable_hashes
                    .get(hash)
                    .cloned()
                    .or_else(|| library::portable_hash(hash))
                    .as_deref()
                    == Some(file)
            })
    }

    /// Ask TonePush for each published tone again, when its answers are old.
    pub(crate) fn refresh_mine(&mut self, ctx: &egui::Context, now: bool) {
        if !crate::account::REACHES_TONEPUSH
            || self.mine.fetching.is_some()
            || self.published.is_empty()
        {
            return;
        }
        if !now
            && self
                .mine
                .fetched
                .is_some_and(|fetched| fetched.elapsed() < FRESH_FOR)
        {
            return;
        }
        let ids: Vec<i64> = self
            .published
            .values()
            .map(|record| record.tone_id)
            .collect();
        let token = self.config.token.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = cloud::CloudClient::new(cloud::site());
            let answers = ids
                .into_iter()
                .map(|id| {
                    let answer =
                        client
                            .tone_as(token.as_deref(), id)
                            .map_err(|error| match error {
                                cloud::ApiError::NotFound(_) => {
                                    "not on TonePush any more".to_owned()
                                }
                                other => other.to_string(),
                            });
                    (id, answer)
                })
                .collect();
            let _ = tx.send(answers);
            ctx.request_repaint();
        });
        self.mine.fetching = Some(rx);
        self.mine.fetched = Some(Instant::now());
    }

    /// Collect TonePush's answers, and those of the search for earlier
    /// publishes.
    pub(crate) fn settle_mine(&mut self) {
        if let Some(answers) = self
            .mine
            .fetching
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.mine.fetching = None;
            for (id, answer) in answers {
                // A failure to reach TonePush keeps what was known.
                match answer {
                    Err(why)
                        if self.mine.details.get(&id).is_some_and(Result::is_ok)
                            && why != "not on TonePush any more" => {}
                    answer => {
                        self.mine.details.insert(id, answer);
                    }
                }
            }
        }
        if let Some(found) = self
            .mine
            .backfill
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.mine.backfill = None;
            let mut added = 0;
            for (series, record) in found {
                if self.published.contains_key(&series) {
                    continue;
                }
                if library::record_published(&series, record.clone()).is_ok() {
                    self.published.insert(series, record);
                    added += 1;
                }
            }
            if added > 0 {
                self.mine.fetched = None;
            }
        }
    }

    /// Find, once, the tones this library published before it kept records:
    /// TonePush has their files, so its feed is searched by name, and only
    /// the exact file, by this account when it is known, is taken.
    pub(crate) fn backfill_mine(&mut self, ctx: &egui::Context) {
        if !crate::account::REACHES_TONEPUSH || self.mine.backfilled || self.mine.backfill.is_some()
        {
            return;
        }
        let Some(files) = self.cloud_files.clone() else {
            return;
        };
        self.mine.backfilled = true;
        let wanted: Vec<(String, String, String, String)> = self
            .lib_entries
            .iter()
            .filter(|entry| !self.published.contains_key(&entry.series))
            .filter_map(|entry| {
                let portable = self.portable_hashes.get(&entry.hash)?;
                files.contains(portable).then(|| {
                    (
                        entry.series.clone(),
                        entry.hash.clone(),
                        entry.name.clone(),
                        portable.clone(),
                    )
                })
            })
            .take(BACKFILL_AT_MOST)
            .collect();
        if wanted.is_empty() {
            return;
        }
        let account = self.config.account.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = cloud::CloudClient::new(cloud::site());
            let _ = tx.send(find_published(&client, &wanted, account.as_deref()));
            ctx.request_repaint();
        });
        self.mine.backfill = Some(rx);
    }

    /// Cloud, Mine: what this library published, with what TonePush says of
    /// each, beside the chosen one's details.
    pub(crate) fn mine_view(&mut self, root: &mut Ui, tier: Tier) {
        let ctx = root.ctx().clone();
        self.settle_mine();
        self.list_account(&ctx, false);
        let account = self.account.tones.is_some();
        if !account {
            self.refresh_mine(&ctx, false);
            self.backfill_mine(&ctx);
        }
        let rows = self.mine_rows();
        let setlists = self.mine_setlists();
        if rows.is_empty() && setlists.is_empty() {
            let rect = root.max_rect();
            let words = if self.mine.backfill.is_some() || self.account.busy() {
                "Looking on TonePush for what you published…"
            } else if account {
                "Nothing published from this account yet. Tones and setlists you publish \
                 appear here, with who can see them and their downloads."
            } else if self.config.token.is_some() {
                "Tones you publish from this library appear here, with their downloads and \
                 versions."
            } else {
                "Sign in to TonePush, and the tones you publish from this library appear here."
            };
            crate::pane::centred(
                root,
                words,
                theme::regular(theme::BODY),
                theme::muted(),
                Pos2::new(rect.center().x, rect.top() + 48.0),
                rect.width() - 40.0,
            );
            return;
        }
        if self
            .mine
            .selected
            .is_none_or(|id| !rows.iter().any(|row| row.record.tone_id == id))
        {
            self.mine.selected = None;
        }
        if self
            .mine
            .selected_setlist
            .is_none_or(|id| !setlists.iter().any(|setlist| setlist.id == id))
        {
            self.mine.selected_setlist = None;
        }
        if tier != Tier::S {
            egui::Panel::right("mine-inspector")
                .resizable(true)
                .default_size(tier.pick(280.0, 300.0, 400.0))
                .size_range(260.0..=520.0)
                .frame(egui::Frame::new().fill(theme::bg()))
                .show(root, |ui| {
                    let rect = ui.max_rect();
                    let mut inner = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    inner.set_clip_rect(rect.intersect(ui.clip_rect()));
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .id_salt("mine-inspector-scroll")
                        .show(&mut inner, |ui| {
                            egui::Frame::new()
                                .inner_margin(egui::Margin {
                                    left: 16,
                                    right: 16,
                                    top: 12,
                                    bottom: 16,
                                })
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    match self.mine.selected_setlist.and_then(|id| {
                                        setlists.iter().find(|setlist| setlist.id == id)
                                    }) {
                                        Some(setlist) => {
                                            self.mine_setlist_inspector(ui, &setlist.clone());
                                        }
                                        None => self.mine_inspector(ui, &rows),
                                    }
                                });
                        });
                    ui.advance_cursor_after_rect(rect);
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::bg()))
            .show(root, |ui| {
                self.mine_head(ui, &rows, &setlists);
                if account {
                    self.mine_account_table(ui, &rows, &setlists, tier);
                } else {
                    self.mine_table(ui, &rows, tier);
                }
            });
    }

    /// The line over the table: how many, and how often downloaded; who is
    /// signed in.
    fn mine_head(&mut self, ui: &mut Ui, rows: &[MineRow], setlists: &[cloud::SetlistSummary]) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 36.0), egui::Sense::hover());
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, theme::line()),
        );
        let downloads: u64 = rows.iter().map(MineRow::downloads).sum();
        let count = rows.len();
        let tones = format!("{count} {}", if count == 1 { "tone" } else { "tones" });
        let downloads = format!(" · {} downloads", crate::format_count(downloads));
        let account = self.account.tones.is_some();
        let name = self
            .account
            .me
            .as_ref()
            .and_then(|me| me.name.clone())
            .or_else(|| self.config.account.clone())
            .unwrap_or_else(|| "You".to_owned());
        let lists = match setlists.len() {
            0 => tones.clone(),
            1 => format!("{tones} and 1 setlist"),
            several => format!("{tones} and {several} setlists"),
        };
        let parts: Vec<(&str, Words)> = if account {
            vec![
                (name.as_str(), Words::Bold),
                (" on TonePush · ", Words::Soft),
                (lists.as_str(), Words::Soft),
                (downloads.as_str(), Words::Soft),
            ]
        } else {
            vec![
                (tones.as_str(), Words::Bold),
                (" published from this library", Words::Soft),
                (downloads.as_str(), Words::Soft),
            ]
        };
        let job = crate::library_view::where_job(&parts, width - 260.0);
        theme::paint_icon(
            ui,
            if account {
                Icon::User
            } else {
                Icon::CloudUpload
            },
            Pos2::new(rect.left() + 24.0, rect.center().y),
            14.0,
            theme::muted(),
        );
        let galley = ui.painter().layout_job(job);
        crate::shell::paint_line(ui, galley, rect.left() + 40.0, rect.center().y);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(Vec2::new(16.0, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let page = self
            .account
            .me
            .as_ref()
            .and_then(|me| me.profile_url.clone())
            .or_else(|| self.config.profile_url.clone());
        match self.config.account.clone() {
            Some(_) if account && page.is_some() => {
                if theme::Button::new("Your page on tonepush.rocks")
                    .ghost()
                    .small()
                    .icon(Icon::ExternalLink)
                    .show(&mut child)
                    .clicked()
                {
                    if let Some(page) = page {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(page));
                    }
                }
            }
            Some(account) if self.config.token.is_some() => {
                theme::label(
                    &mut child,
                    &format!("Signed in as {account}"),
                    theme::regular(12.5),
                    theme::muted(),
                );
            }
            _ => {
                if self.signing_in.is_none()
                    && theme::Button::new("Sign in")
                        .ghost()
                        .small()
                        .show(&mut child)
                        .clicked()
                {
                    self.start_signing_in(ui.ctx());
                }
            }
        }
        if self.mine.fetching.is_some() || self.account.busy() {
            let (spot, _) = child.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
            crate::shell::spin(&child, spot.center(), 5.0);
        }
        if let Some(problem) = self.account.problem.clone() {
            theme::label(&mut child, &problem, theme::regular(12.5), theme::danger());
        }
    }

    /// The table of what was published.
    fn mine_table(&mut self, ui: &mut Ui, rows: &[MineRow], tier: Tier) {
        let device = self.device.trim().to_owned();
        let columns = vec![
            table::Column::new("", 44.0),
            table::Column::new("Pedal", tier.pick(44.0, 88.0, 100.0)),
            table::Column::new("Name", 150.0),
            table::Column::new("Song · Artist", 150.0).fills(),
            table::Column::new("On TonePush", 150.0),
            table::Column::new("Downloads", 80.0),
            table::Column::new("Published", 70.0),
        ];
        let mut grid = table::Grid {
            columns,
            sort: self.mine.sort,
            sticky: 3,
            row_height: crate::library_pane::TABLE_ROW,
            header_height: crate::library_pane::TABLE_HEADER,
            nothing_yet: "Nothing published from this library yet.",
            draggable: false,
            click_plays: true,
            ..Default::default()
        };
        for row in rows {
            let discovered = row.discovered();
            let local = row.local.map(|index| &self.lib_entries[index]);
            let found_marker = discovered
                .as_ref()
                .and_then(Self::cloud_marker)
                .or_else(|| local.and_then(|entry| entry.marker.clone()));
            let plays = discovered
                .as_ref()
                .is_none_or(|tone| self.cloud_audition_blocker(tone).is_none());
            let on_pedal = local
                .and_then(|entry| self.slot_holding(&entry.hash))
                .map(|slot| self.active_slot_label(slot));
            let places = vec![
                (
                    if plays { Icon::Pedal } else { Icon::Ban },
                    if on_pedal.is_some() {
                        theme::Sync::Same
                    } else {
                        theme::Sync::Absent
                    },
                    match &on_pedal {
                        Some(slot) => format!("On the {device} at {slot}"),
                        None if plays => format!("Play it on the {device}"),
                        None => "The pedal connected cannot play it".to_owned(),
                    },
                    plays,
                ),
                (
                    Icon::Computer,
                    if local.is_some() {
                        theme::Sync::Same
                    } else {
                        theme::Sync::Absent
                    },
                    if local.is_some() {
                        "In your library".to_owned()
                    } else {
                        "Not in this library".to_owned()
                    },
                    true,
                ),
            ];
            let marker_cell = match &found_marker {
                Some(found) => table::Cell::Marker {
                    family: found.family.label(),
                    model: found.model.clone(),
                    solid: plays,
                    compact: tier == Tier::S,
                    hover: format!("For {}", found.with_article()),
                },
                None => table::Cell::Text(String::new()),
            };
            let (song, artist) = match discovered.as_ref() {
                Some(tone) if tone.song.kind == cloud::SongKind::Song => (
                    tone.song.title.clone(),
                    tone.song.artist.clone().unwrap_or_default(),
                ),
                Some(_) => ("Original".to_owned(), String::new()),
                None => local.map_or((String::new(), String::new()), |entry| {
                    if entry.meta.song.trim().is_empty() {
                        ("Original".to_owned(), String::new())
                    } else {
                        (entry.meta.song.clone(), entry.meta.artist.clone())
                    }
                }),
            };
            let on_tonepush = match (row.versions(), &row.problem) {
                (Some((current, _)), _) => match self.mine_ahead(row) {
                    Some(ahead) => {
                        table::Cell::Hot(format!("v{current} · library has v{}", ahead.version))
                    }
                    None if local.is_none() => {
                        table::Cell::Dim(format!("v{current} · not in this library"))
                    }
                    None => table::Cell::Text(format!("v{current}")),
                },
                (None, Some(problem)) => table::Cell::Dim(problem.clone()),
                (None, None) => table::Cell::Dim("asking TonePush…".to_owned()),
            };
            let downloads = row.downloads();
            let published = row.published_at();
            grid.rows.push(vec![
                table::Cell::Places(places),
                marker_cell,
                table::Cell::Name {
                    text: row.name(),
                    tag: None,
                },
                table::Cell::Pair {
                    text: song,
                    aside: artist,
                },
                on_tonepush,
                table::Cell::Value {
                    text: crate::format_count(downloads),
                    key: format!("{downloads:012}"),
                    dim: row.details.is_none(),
                },
                table::Cell::Value {
                    text: crate::day_month(&published),
                    key: published.clone(),
                    dim: false,
                },
            ]);
            grid.chosen.push(false);
        }
        grid.selected = self
            .mine
            .selected
            .and_then(|id| rows.iter().position(|row| row.record.tone_id == id));
        let loading = self.heard_loading();
        grid.playing = rows
            .iter()
            .position(|row| {
                self.hears(&crate::audition::Source::TonePush(row.record.tone_id))
                    || row.details.as_ref().is_some_and(|tone| {
                        self.hears(&crate::audition::Source::TonePush(tone.summary.id))
                    })
            })
            .map(|row| (row, loading));
        let order = grid.sort_rows();
        let sorted: Vec<&MineRow> = order.iter().map(|&row| &rows[row]).collect();
        self.mine.order = sorted.iter().map(|row| row.record.tone_id).collect();
        if std::mem::take(&mut self.mine.reveal) {
            grid.reveal = grid.selected;
            grid.reveal_near = true;
        }

        let did = table::show(ui, "mine", &mut grid);
        self.menu_anchor = did
            .selected_rect
            .map(|rect| rect.left_bottom() + egui::vec2(36.0, 2.0));
        if let Some(column) = did.sort {
            self.mine.sort = if column == self.mine.sort.0 {
                (column, !self.mine.sort.1)
            } else {
                (column, column != DOWNLOADS)
            };
        }
        let ctx = ui.ctx().clone();
        if let Some((row, ..)) = did.clicked {
            if let Some(row) = sorted.get(row) {
                self.mine.selected = Some(row.record.tone_id);
                self.hearing.arrows = Arrows::Mine;
                self.mine_play(row.record.tone_id, &ctx);
            }
        }
        if let Some(row) = did.context.and_then(|row| sorted.get(row)) {
            let at = ctx
                .input(|input| input.pointer.interact_pos())
                .unwrap_or_default();
            self.mine.selected = Some(row.record.tone_id);
            self.hearing.arrows = Arrows::Mine;
            self.open_row_menu(&ctx, crate::menus::MenuFor::Mine(row.record.tone_id), at);
        }
        if let Some((row, place)) = did.place {
            if let Some(row) = sorted.get(row) {
                self.mine.selected = Some(row.record.tone_id);
                self.hearing.arrows = Arrows::Mine;
                match place {
                    0 => self.mine_play(row.record.tone_id, &ctx),
                    _ => self.mine_show_or_keep(row.record.tone_id, &ctx),
                }
            }
        }
        let _ = (PLACES, PEDAL, NAME, SONG, ON_TONEPUSH, PUBLISHED);
    }

    /// The row Mine has for a tone on TonePush.
    pub(crate) fn mine_row(&self, tone_id: i64) -> Option<MineRow> {
        self.mine_rows()
            .into_iter()
            .find(|row| row.record.tone_id == tone_id)
    }

    /// Play one of your tones on the pedal, as TonePush has it.
    pub(crate) fn mine_play(&mut self, tone_id: i64, ctx: &egui::Context) {
        let Some(row) = self.mine_row(tone_id) else {
            return;
        };
        match row.discovered() {
            Some(tone) => self.start_cloud_entry_action(tone, CloudAction::Audition, ctx),
            None => self.note(format!(
                "{} cannot be played until TonePush answers for it",
                row.name()
            )),
        }
    }

    /// Show one of your tones in the library, or keep it there when this
    /// library no longer holds it.
    pub(crate) fn mine_show_or_keep(&mut self, tone_id: i64, ctx: &egui::Context) {
        let Some(row) = self.mine_row(tone_id) else {
            return;
        };
        match (row.local, row.discovered()) {
            (Some(index), _) => {
                let tier = Tier::now(ctx);
                self.library_device_filter = None;
                self.open_library(crate::LibraryView::Tones, tier);
                self.choose_tone(index);
                self.lib_reveal = true;
            }
            (None, Some(tone)) => {
                self.start_cloud_entry_action(tone, CloudAction::Computer, ctx);
            }
            (None, None) => {}
        }
    }

    /// Step through Mine's rows, playing each.
    pub(crate) fn step_mine(&mut self, direction: i64, ctx: &egui::Context) {
        let order = self.mine.order.clone();
        if order.is_empty() || direction == 0 {
            return;
        }
        let at = self
            .mine
            .selected
            .and_then(|id| order.iter().position(|known| *known == id));
        let next = match at {
            None => 0,
            Some(at) => (at as i64 + direction).clamp(0, order.len() as i64 - 1) as usize,
        };
        if Some(next) == at {
            return;
        }
        self.mine.selected = Some(order[next]);
        self.mine.reveal = true;
        self.mine_play(order[next], ctx);
    }

    /// The chosen tone's details: where it is, what TonePush says of it, and
    /// its versions there.
    fn mine_inspector(&mut self, ui: &mut Ui, rows: &[MineRow]) {
        let Some(row) = self
            .mine
            .selected
            .and_then(|id| rows.iter().find(|row| row.record.tone_id == id))
            .cloned()
        else {
            ui.add_space(24.0);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Choose a tone to see what TonePush says of it, and its versions there.",
                    )
                    .font(theme::regular(theme::SECONDARY))
                    .color(theme::muted()),
                )
                .wrap(),
            );
            return;
        };
        let ctx = ui.ctx().clone();
        theme::label(ui, &row.name(), theme::semibold(17.0), theme::text());
        ui.add_space(4.0);
        let discovered = row.discovered();
        let found_marker = discovered
            .as_ref()
            .and_then(Self::cloud_marker)
            .or_else(|| row.local.and_then(|i| self.lib_entries[i].marker.clone()));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            marker(ui, found_marker.as_ref(), true);
            let mut words = Vec::new();
            if let Some((current, _)) = row.versions() {
                words.push(format!("v{current} on TonePush"));
            }
            if let Some(tone) = discovered.as_ref() {
                words.push(if tone.song.kind == cloud::SongKind::Song {
                    tone.song.title.clone()
                } else {
                    "Original".to_owned()
                });
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(words.join(" · "))
                        .font(theme::regular(12.5))
                        .color(theme::muted()),
                )
                .truncate(),
            );
        });
        ui.add_space(12.0);
        crate::library_view::rule(ui);
        ui.add_space(6.0);

        // The library.
        let ahead = self.mine_ahead(&row);
        match (row.local, &ahead) {
            (Some(_), Some(ahead)) => {
                let label = format!("Publish v{}", row.versions().map_or(1, |(_, n)| n + 1));
                let version = format!("v{}", ahead.version);
                if where_row(
                    ui,
                    Icon::Computer,
                    theme::hot(),
                    &[("Your library has ", Words::Soft), (&version, Words::Hot)],
                    Some(&label),
                ) {
                    self.ask_to_publish(vec![ahead.hash.clone()]);
                }
            }
            (Some(index), None) => {
                let version = format!("v{}", self.lib_entries[index].version);
                if where_row(
                    ui,
                    Icon::Computer,
                    theme::text_soft(),
                    &[
                        ("In your library as ", Words::Soft),
                        (&version, Words::Bold),
                        (", the same", Words::Soft),
                    ],
                    Some("Show"),
                ) {
                    self.mine_show_or_keep(row.record.tone_id, &ctx);
                }
            }
            (None, _) => {
                if where_row(
                    ui,
                    Icon::Computer,
                    theme::muted(),
                    &[("Not in this library", Words::Soft)],
                    discovered.is_some().then_some("Keep"),
                ) {
                    self.mine_show_or_keep(row.record.tone_id, &ctx);
                }
            }
        }
        // Downloads, and when.
        let downloads = format!("{} downloads", crate::format_count(row.downloads()));
        let when = format!(" · published {}", crate::day_month(&row.published_at()));
        where_row(
            ui,
            Icon::Download,
            theme::text_soft(),
            &[(&downloads, Words::Soft), (&when, Words::Soft)],
            None,
        );
        // Who sees it, and changing that, where the server can.
        if let Some(visibility) = self.tone_visibility(row.record.tone_id) {
            let (icon, words, action, other) = match visibility {
                cloud::Visibility::Everyone => (
                    Icon::Globe,
                    "Everyone can see it",
                    "Hide",
                    cloud::Visibility::OnlyYou,
                ),
                cloud::Visibility::OnlyYou => (
                    Icon::EyeOff,
                    "Only you can see it",
                    "Show",
                    cloud::Visibility::Everyone,
                ),
            };
            if where_row(
                ui,
                icon,
                theme::text_soft(),
                &[(words, Words::Soft)],
                Some(action),
            ) {
                self.set_tone_visibility(row.record.tone_id, other, &ctx);
            }
        } else {
            match &row.problem {
                Some(problem) => {
                    where_row(
                        ui,
                        Icon::CircleAlert,
                        theme::muted(),
                        &[(&capital(problem), Words::Soft)],
                        None,
                    );
                }
                None => {
                    let open = where_row(
                        ui,
                        Icon::Globe,
                        theme::text_soft(),
                        &[("Public on tonepush.rocks", Words::Soft)],
                        row.details.as_ref().and(Some("Open")),
                    );
                    if open {
                        if let Some(url) = self.mine_url(&row) {
                            ctx.open_url(egui::OpenUrl::new_tab(url));
                        }
                    }
                }
            }
        }
        // The pedal.
        let device = self.device.trim().to_owned();
        let on_pedal = row
            .local
            .and_then(|index| self.slot_holding(&self.lib_entries[index].hash))
            .map(|slot| self.active_slot_label(slot));
        if self.pedal_online() {
            match on_pedal {
                Some(slot) => {
                    let slot = format!("On the {device} at {slot}");
                    where_row(
                        ui,
                        Icon::Pedal,
                        theme::text_soft(),
                        &[(&slot, Words::Soft)],
                        None,
                    );
                }
                None => {
                    let words = format!("Not on the {device}");
                    where_row(
                        ui,
                        Icon::Pedal,
                        theme::muted(),
                        &[(&words, Words::Soft)],
                        None,
                    );
                }
            }
        }

        if self.account.lists_tones() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if let Some(url) = self.mine_url(&row) {
                    if theme::Button::new("Open")
                        .small()
                        .icon(Icon::ExternalLink)
                        .show(ui)
                        .clicked()
                    {
                        ctx.open_url(egui::OpenUrl::new_tab(url));
                    }
                }
                if theme::Button::new("Delete…")
                    .ghost()
                    .small()
                    .icon(Icon::Remove)
                    .show(ui)
                    .clicked()
                {
                    self.account.confirm_delete =
                        Some(crate::account::Yours::Tone(row.record.tone_id));
                }
            });
        }

        // Its versions on TonePush, a click on one playing it.
        let Some(tone) = discovered else {
            return;
        };
        if tone.tone.versions.is_empty() {
            return;
        }
        ui.add_space(10.0);
        section(ui, "Versions on TonePush");
        let loading = self.heard_loading();
        let mut chosen = None;
        for version in tone.tone.versions.iter().rev() {
            let key = if version.current {
                tone.tone.summary.id
            } else {
                Self::cloud_version_entry(&tone, version).tone.summary.id
            };
            let playing = self
                .hears(&crate::audition::Source::TonePush(key))
                .then_some(loading);
            let clicked = version_row(
                ui,
                &format!("v{}", version.number),
                &crate::day_month(&version.created_at),
                playing,
                |ui| {
                    if version.current {
                        theme::Chip::new("Current").mood(theme::Mood::Ok).show(ui);
                    }
                },
            );
            if clicked {
                chosen = Some(version.clone());
            }
        }
        if let Some(version) = chosen {
            let entry = if version.current {
                tone
            } else {
                Self::cloud_version_entry(&tone, &version)
            };
            self.start_cloud_entry_action(entry, CloudAction::Audition, &ctx);
        }
    }

    /// The table of everything you published, setlists first, each group
    /// under its caption (sheet 19).
    fn mine_account_table(
        &mut self,
        ui: &mut Ui,
        rows: &[MineRow],
        setlists: &[cloud::SetlistSummary],
        tier: Tier,
    ) {
        let columns = vec![
            table::Column::new("", 44.0),
            table::Column::new("Pedal", tier.pick(44.0, 88.0, 100.0)),
            table::Column::new("Name", 170.0),
            table::Column::new("On TonePush", 150.0).fills(),
            table::Column::new("Who sees it", 110.0),
            table::Column::new("Downloads", 80.0),
            table::Column::new("Published", 70.0),
        ];
        let mut grid = table::Grid {
            columns,
            sort: (ACCOUNT_DOWNLOADS, false),
            sticky: 3,
            row_height: crate::library_pane::TABLE_ROW,
            header_height: crate::library_pane::TABLE_HEADER,
            nothing_yet: "Nothing published yet.",
            click_plays: true,
            ..Default::default()
        };
        // Downloads, most first, in each group.
        let mut tones: Vec<&MineRow> = rows.iter().collect();
        tones.sort_by_key(|row| std::cmp::Reverse(row.downloads()));
        let mut items = Vec::new();
        if !setlists.is_empty() {
            items.push(Item::Caption(format!("Setlists  {}", setlists.len())));
            items.extend(setlists.iter().map(Item::Setlist));
            items.push(Item::Caption(format!("Tones  {}", tones.len())));
        }
        items.extend(tones.iter().map(|row| Item::Tone(row)));
        for item in &items {
            grid.rows.push(match item {
                Item::Caption(words) => vec![
                    table::Cell::Group(words.clone()),
                    table::Cell::Text(String::new()),
                    table::Cell::Text(String::new()),
                    table::Cell::Text(String::new()),
                    table::Cell::Text(String::new()),
                    table::Cell::Text(String::new()),
                    table::Cell::Text(String::new()),
                ],
                Item::Setlist(setlist) => self.mine_setlist_cells(setlist, tier),
                Item::Tone(row) => self.mine_account_cells(row, tier),
            });
            grid.chosen.push(false);
        }
        grid.selected = items.iter().position(|item| match item {
            Item::Setlist(setlist) => self.mine.selected_setlist == Some(setlist.id),
            Item::Tone(row) => {
                self.mine.selected_setlist.is_none()
                    && self.mine.selected == Some(row.record.tone_id)
            }
            Item::Caption(_) => false,
        });
        let loading = self.heard_loading();
        grid.playing = items
            .iter()
            .position(|item| match item {
                Item::Tone(row) => row.details.as_ref().is_some_and(|tone| {
                    self.hears(&crate::audition::Source::TonePush(tone.summary.id))
                }),
                _ => false,
            })
            .map(|row| (row, loading));
        self.mine.order = tones.iter().map(|row| row.record.tone_id).collect();
        if std::mem::take(&mut self.mine.reveal) {
            grid.reveal = grid.selected;
            grid.reveal_near = true;
        }

        let did = table::show(ui, "mine-account", &mut grid);
        self.menu_anchor = did
            .selected_rect
            .map(|rect| rect.left_bottom() + egui::vec2(36.0, 2.0));
        let ctx = ui.ctx().clone();
        let pointer = ctx
            .input(|input| input.pointer.interact_pos())
            .unwrap_or_default();
        if let Some((row, ..)) = did.clicked {
            match items.get(row) {
                Some(Item::Tone(row)) => {
                    self.mine.selected = Some(row.record.tone_id);
                    self.mine.selected_setlist = None;
                    self.hearing.arrows = Arrows::Mine;
                    self.mine_play(row.record.tone_id, &ctx);
                }
                Some(Item::Setlist(setlist)) => {
                    self.mine.selected_setlist = Some(setlist.id);
                    self.hearing.arrows = Arrows::Mine;
                }
                _ => {}
            }
        }
        if let Some(row) = did.context {
            match items.get(row) {
                Some(Item::Tone(row)) => {
                    self.mine.selected = Some(row.record.tone_id);
                    self.mine.selected_setlist = None;
                    self.hearing.arrows = Arrows::Mine;
                    self.open_row_menu(
                        &ctx,
                        crate::menus::MenuFor::Mine(row.record.tone_id),
                        pointer,
                    );
                }
                Some(Item::Setlist(setlist)) => {
                    self.mine.selected_setlist = Some(setlist.id);
                    self.hearing.arrows = Arrows::Mine;
                    self.open_row_menu(
                        &ctx,
                        crate::menus::MenuFor::MineSetlist(setlist.id),
                        pointer,
                    );
                }
                _ => {}
            }
        }
        if let Some((row, place)) = did.place {
            if let Some(Item::Tone(row)) = items.get(row) {
                self.mine.selected = Some(row.record.tone_id);
                self.mine.selected_setlist = None;
                self.hearing.arrows = Arrows::Mine;
                match place {
                    0 => self.mine_play(row.record.tone_id, &ctx),
                    _ => self.mine_show_or_keep(row.record.tone_id, &ctx),
                }
            }
        }
    }

    /// One of your tones as a row of the account's table.
    fn mine_account_cells(&self, row: &MineRow, tier: Tier) -> Vec<table::Cell> {
        let device = self.device.trim().to_owned();
        let discovered = row.discovered();
        let local = row.local.map(|index| &self.lib_entries[index]);
        let found_marker = discovered
            .as_ref()
            .and_then(Self::cloud_marker)
            .or_else(|| local.and_then(|entry| entry.marker.clone()));
        let plays = discovered
            .as_ref()
            .is_none_or(|tone| self.cloud_audition_blocker(tone).is_none());
        let places = vec![
            (
                if plays { Icon::Pedal } else { Icon::Ban },
                theme::Sync::Absent,
                if plays {
                    format!("Play it on the {device}")
                } else {
                    "The pedal connected cannot play it".to_owned()
                },
                plays,
            ),
            (
                Icon::Computer,
                if local.is_some() {
                    theme::Sync::Same
                } else {
                    theme::Sync::Absent
                },
                if local.is_some() {
                    "In your library".to_owned()
                } else {
                    "Not in this library: keep it".to_owned()
                },
                true,
            ),
        ];
        let on_tonepush = match row.versions() {
            Some((current, _)) => match self.mine_ahead(row) {
                Some(ahead) => {
                    table::Cell::Hot(format!("v{current} · library has v{}", ahead.version))
                }
                None if local.is_none() => {
                    table::Cell::Dim(format!("v{current} · not in this library"))
                }
                None => table::Cell::Text(format!("v{current}")),
            },
            None => table::Cell::Dim(String::new()),
        };
        let visibility = self.tone_visibility(row.record.tone_id);
        let published = row.published_at();
        vec![
            table::Cell::Places(places),
            marker_cell(found_marker.as_ref(), plays, tier),
            table::Cell::Name {
                text: row.name(),
                tag: None,
            },
            on_tonepush,
            who_sees(visibility),
            table::Cell::Value {
                text: crate::format_count(row.downloads()),
                key: format!("{:012}", row.downloads()),
                dim: false,
            },
            table::Cell::Value {
                text: crate::day_month(&published),
                key: published,
                dim: false,
            },
        ]
    }

    /// One of your setlists as a row of the account's table.
    fn mine_setlist_cells(&self, setlist: &cloud::SetlistSummary, tier: Tier) -> Vec<table::Cell> {
        let found_marker = crate::devices::Marker::of_device(&setlist.device.name, "");
        let local = self.local_setlist_named(&setlist.name);
        let places = vec![
            (
                Icon::ListMusic,
                theme::Sync::Unknown,
                "A setlist".to_owned(),
                false,
            ),
            (
                Icon::Computer,
                if local.is_some() {
                    theme::Sync::Same
                } else {
                    theme::Sync::Absent
                },
                if local.is_some() {
                    "In your library".to_owned()
                } else {
                    "Not in this library".to_owned()
                },
                false,
            ),
        ];
        let presets = format!("{} presets", setlist.slot_count);
        let on_tonepush = match local {
            Some(index) => format!("v{} · {presets}", self.lib_setlists[index].1.revision()),
            None => presets,
        };
        let visibility = match setlist.visibility.as_str() {
            "only_you" => cloud::Visibility::OnlyYou,
            _ => cloud::Visibility::Everyone,
        };
        vec![
            table::Cell::Places(places),
            marker_cell(found_marker.as_ref(), true, tier),
            table::Cell::Name {
                text: setlist.name.clone(),
                tag: None,
            },
            table::Cell::Dim(on_tonepush),
            who_sees(Some(visibility)),
            table::Cell::Dim(String::new()),
            table::Cell::Value {
                text: crate::day_month(&setlist.created_at),
                key: setlist.created_at.clone(),
                dim: false,
            },
        ]
    }

    /// One of your setlists' details: who can see it, the library's copy,
    /// and putting it on the pedal.
    fn mine_setlist_inspector(&mut self, ui: &mut Ui, setlist: &cloud::SetlistSummary) {
        let ctx = ui.ctx().clone();
        theme::label(ui, &setlist.name, theme::semibold(17.0), theme::text());
        ui.add_space(4.0);
        let found_marker = crate::devices::Marker::of_device(&setlist.device.name, "");
        let local = self.local_setlist_named(&setlist.name);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            marker(ui, found_marker.as_ref(), true);
            let mut words = Vec::new();
            if let Some(index) = local {
                words.push(format!("v{}", self.lib_setlists[index].1.revision()));
            }
            words.push(format!("{} presets", setlist.slot_count));
            if let Some(venue) = setlist.venue.as_deref().filter(|venue| !venue.is_empty()) {
                words.push(venue.to_owned());
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(words.join(" · "))
                        .font(theme::regular(12.5))
                        .color(theme::muted()),
                )
                .truncate(),
            );
        });
        ui.add_space(12.0);
        crate::library_view::rule(ui);
        ui.add_space(6.0);
        let (icon, words, action, other) = match setlist.visibility.as_str() {
            "only_you" => (
                Icon::EyeOff,
                "Only you can see it",
                "Show",
                cloud::Visibility::Everyone,
            ),
            _ => (
                Icon::Globe,
                "Everyone can see it",
                "Hide",
                cloud::Visibility::OnlyYou,
            ),
        };
        if where_row(
            ui,
            icon,
            theme::text_soft(),
            &[(words, Words::Soft)],
            Some(action),
        ) {
            self.set_setlist_visibility(setlist.id, other, &ctx);
        }
        let when = format!("Published {}", crate::day_month(&setlist.created_at));
        where_row(
            ui,
            Icon::CloudUpload,
            theme::text_soft(),
            &[(&when, Words::Soft)],
            None,
        );
        match local {
            Some(index) => {
                let version = format!("v{}", self.lib_setlists[index].1.revision());
                if where_row(
                    ui,
                    Icon::Computer,
                    theme::text_soft(),
                    &[
                        ("In your library as ", Words::Soft),
                        (&version, Words::Bold),
                    ],
                    Some("Show"),
                ) {
                    let tier = Tier::now(&ctx);
                    self.open_library(crate::LibraryView::Setlists, tier);
                    self.select_setlist_entry(index);
                }
                if let Some(states) = self.setlist_states(&self.lib_setlists[index].1.clone()) {
                    let differing = states.iter().filter(|state| state.differs()).count();
                    let device = self.device.trim().to_owned();
                    let words = match differing {
                        0 => format!("Matches the {device}"),
                        1 => format!("1 slot differs from the {device}"),
                        n => format!("{n} slots differ from the {device}"),
                    };
                    where_row(
                        ui,
                        Icon::Pedal,
                        if differing > 0 {
                            theme::hot()
                        } else {
                            theme::text_soft()
                        },
                        &[(&words, Words::Soft)],
                        None,
                    );
                }
            }
            None => {
                where_row(
                    ui,
                    Icon::Computer,
                    theme::muted(),
                    &[("Not in this library", Words::Soft)],
                    None,
                );
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let device = self.device.trim().to_owned();
            if let Some(index) = local {
                if theme::Button::new(&format!("Put on {device}…"))
                    .small()
                    .icon(Icon::Download)
                    .enabled(self.pedal_online())
                    .show(ui)
                    .clicked()
                {
                    self.select_setlist_entry(index);
                    self.confirm_push = Some(index);
                }
            }
            if theme::Button::new("Delete…")
                .ghost()
                .small()
                .icon(Icon::Remove)
                .show(ui)
                .clicked()
            {
                self.account.confirm_delete = Some(crate::account::Yours::Setlist(setlist.id));
            }
        });
        ui.add_space(10.0);
        section(ui, "Its tones");
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!(
                    "{} presets, each a tone you published, in the order the setlist \
                     plays them.",
                    setlist.slot_count
                ))
                .font(theme::regular(theme::SECONDARY))
                .color(theme::text_soft()),
            )
            .wrap(),
        );
    }

    /// The tone's page on TonePush.
    pub(crate) fn mine_url(&self, row: &MineRow) -> Option<String> {
        let file = row.details.as_ref()?.file_sha256.clone()?;
        Some(cloud::tone_url(&file))
    }

    /// Mine's keys, on its rows: the arrows step and play, Space plays or
    /// puts back, Ctrl Enter puts in a slot, F2 renames on TonePush and
    /// Shift F10 opens the menu.
    pub(crate) fn mine_keys(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let consume = |modifiers: Modifiers, key: Key| {
            ctx.input_mut(|input| input.consume_key(modifiers, key))
        };
        if consume(Modifiers::NONE, Key::ArrowDown) {
            self.step_mine(1, ctx);
        } else if consume(Modifiers::NONE, Key::ArrowUp) {
            self.step_mine(-1, ctx);
        } else if consume(Modifiers::NONE, Key::Space) {
            let Some(id) = self.mine.selected else {
                return;
            };
            let playing = self.mine_row(id).is_some_and(|row| {
                row.details.as_ref().is_some_and(|tone| {
                    self.hears(&crate::audition::Source::TonePush(tone.summary.id))
                })
            });
            if playing {
                self.put_back();
            } else {
                self.mine_play(id, ctx);
            }
        }
    }
}

/// Search TonePush's feed for tones this library published before it kept
/// records: by name, taking only the exact file, by this account when it is
/// known. Answers each one found as the record it would have kept.
pub(crate) fn find_published(
    client: &cloud::CloudClient,
    wanted: &[(String, String, String, String)],
    account: Option<&str>,
) -> Vec<(String, library::Published)> {
    let mut found = Vec::new();
    for (series, hash, name, file) in wanted {
        let Ok(page) = client.discover_page(name, cloud::DiscoveryOrder::Newest, None, 1) else {
            continue;
        };
        let matching = page.entries.into_iter().find(|entry| {
            let tone = &entry.tone;
            let same_file = tone.file_sha256.as_deref() == Some(file.as_str())
                || tone
                    .versions
                    .iter()
                    .any(|version| &version.file_sha256 == file);
            let same_account = account.is_none_or(|account| {
                tone.summary
                    .creator
                    .as_deref()
                    .is_none_or(|creator| creator.trim().eq_ignore_ascii_case(account.trim()))
            });
            same_file && same_account
        });
        if let Some(entry) = matching {
            found.push((
                series.clone(),
                library::Published {
                    tone_id: crate::publish::stable_id(&entry.tone.summary),
                    song_id: entry.tone.summary.song_id,
                    hash: hash.clone(),
                    name: entry.tone.summary.name.clone(),
                    at: entry.tone.summary.created_at.clone(),
                },
            ));
        }
    }
    found
}

/// What the account's table holds, in order.
enum Item<'a> {
    Caption(String),
    Setlist(&'a cloud::SetlistSummary),
    Tone(&'a MineRow),
}

/// The Downloads column of the account's table.
const ACCOUNT_DOWNLOADS: usize = 5;

/// A tone's or setlist's marker as a table cell.
fn marker_cell(found: Option<&crate::devices::Marker>, plays: bool, tier: Tier) -> table::Cell {
    match found {
        Some(found) => table::Cell::Marker {
            family: found.family.label(),
            model: found.model.clone(),
            solid: plays,
            compact: tier == Tier::S,
            hover: format!("For {}", found.with_article()),
        },
        None => table::Cell::Text(String::new()),
    }
}

/// Who can see something on TonePush, as a table cell.
fn who_sees(visibility: Option<cloud::Visibility>) -> table::Cell {
    match visibility {
        Some(cloud::Visibility::OnlyYou) => table::Cell::Seen {
            icon: Icon::EyeOff,
            text: "Only you".to_owned(),
        },
        Some(cloud::Visibility::Everyone) => table::Cell::Seen {
            icon: Icon::Globe,
            text: "Everyone".to_owned(),
        },
        None => table::Cell::Dim(String::new()),
    }
}

/// "not on TonePush any more" as a sentence on its own: "Not on TonePush any
/// more".
fn capital(words: &str) -> String {
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[allow(dead_code)]
fn unused(_: Rect) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::tests::{tone_json, StubServer};
    use crate::session::{Cmd, Evt};
    use crate::LibEntry;
    use std::sync::mpsc;

    fn app() -> (App, mpsc::Sender<Evt>, mpsc::Receiver<Cmd>) {
        let (to_device, cmds) = mpsc::channel();
        let (events, from_device) = mpsc::channel();
        let app = App::new(&egui::Context::default(), to_device, from_device);
        let _ = cmds.try_iter().count();
        (app, events, cmds)
    }

    fn entry(name: &str, series: &str, hash: &str, version: u32) -> LibEntry {
        LibEntry {
            hash: hash.to_owned(),
            series: series.to_owned(),
            name: name.to_owned(),
            line: String::new(),
            meta: library::Meta::default(),
            added_at: String::new(),
            modified_at: String::new(),
            downloads: None,
            rating: None,
            version,
            versions: version,
            chain: Vec::new(),
            pro: false,
            marker: Some(crate::devices::Marker::hx("Stomp")),
            firmware: "3.80".to_owned(),
        }
    }

    fn details(id: i64, file: &str, current: u32, downloads: u64) -> cloud::ToneDetails {
        let mut tone: cloud::ToneDetails =
            serde_json::from_value(tone_json(id, 12, "Slapback Twang")).unwrap();
        tone.summary.name = "Slapback Twang".to_owned();
        tone.file_sha256 = Some(file.to_owned());
        tone.summary.version_number = Some(current);
        tone.summary.versions_count = current;
        tone.summary.downloads_count = Some(downloads);
        tone
    }

    /// A published tone's row says which version TonePush gives, and that
    /// the library has moved on when its file is not that one.
    #[test]
    fn a_row_says_when_the_library_has_moved_on() {
        let _scratch = library::tests::Scratch::new("mine-ahead");
        let (mut app, _events, _cmds) = app();
        app.lib_entries = vec![entry("Slapback Twang", "series-slap", "local-v3", 3)];
        app.portable_hashes
            .insert("local-v3".to_owned(), "file-v3".to_owned());
        app.published.insert(
            "series-slap".to_owned(),
            library::Published {
                tone_id: 34,
                song_id: 12,
                hash: "local-v2".to_owned(),
                name: "Slapback Twang".to_owned(),
                at: "2026-09-17T10:00:00Z".to_owned(),
            },
        );
        app.mine
            .details
            .insert(34, Ok(details(34, "file-v2", 2, 312)));

        let rows = app.mine_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].local, Some(0));
        assert_eq!(rows[0].versions(), Some((2, 2)));
        assert_eq!(rows[0].downloads(), 312);
        let ahead = app.mine_ahead(&rows[0]).expect("the library has v3");
        assert_eq!((ahead.hash.as_str(), ahead.version), ("local-v3", 3));

        // Once TonePush gives the library's file, nothing is ahead.
        app.mine
            .details
            .insert(34, Ok(details(34, "file-v3", 3, 312)));
        assert!(app.mine_ahead(&app.mine_rows()[0]).is_none());
    }

    /// A tone TonePush no longer has says so, and keeps its place in Mine.
    #[test]
    fn a_tone_gone_from_tonepush_says_so() {
        let _scratch = library::tests::Scratch::new("mine-gone");
        let (mut app, _events, _cmds) = app();
        app.published.insert(
            "series-gone".to_owned(),
            library::Published {
                tone_id: 77,
                song_id: 5,
                name: "Old Lead".to_owned(),
                ..Default::default()
            },
        );
        app.mine
            .details
            .insert(77, Err("not on TonePush any more".to_owned()));
        let rows = app.mine_rows();
        assert_eq!(rows[0].name(), "Old Lead");
        assert_eq!(rows[0].problem.as_deref(), Some("not on TonePush any more"));
        assert!(rows[0].discovered().is_none());
    }

    /// Tones published before records were kept are found by name, and
    /// only by the exact file from this account: a mocked feed.
    #[test]
    fn earlier_publishes_are_found_by_the_exact_file() {
        let mut ours = tone_json(34, 12, "Slapback Twang");
        ours["name"] = "Slapback Twang".into();
        ours["creator"] = "Noa Calder".into();
        ours["file_sha256"] = "f".repeat(64).into();
        let mut theirs = tone_json(35, 13, "Slapback Twang");
        theirs["name"] = "Slapback Twang".into();
        theirs["creator"] = "Someone Else".into();
        theirs["file_sha256"] = "f".repeat(64).into();
        let server = StubServer::start(vec![
            (
                200,
                serde_json::json!({"tones": [theirs, ours], "total": 2}),
            ),
            (200, serde_json::json!({"tones": [], "total": 0})),
        ]);
        let client = cloud::CloudClient::new(&server.base);
        let wanted = vec![
            (
                "series-slap".to_owned(),
                "local".to_owned(),
                "Slapback Twang".to_owned(),
                "f".repeat(64),
            ),
            (
                "series-none".to_owned(),
                "other".to_owned(),
                "Nothing Here".to_owned(),
                "0".repeat(64),
            ),
        ];
        let found = find_published(&client, &wanted, Some("Noa Calder"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "series-slap");
        assert_eq!((found[0].1.tone_id, found[0].1.song_id), (34, 12));
        let requests = server.finish();
        assert!(String::from_utf8_lossy(&requests[0]).starts_with("GET /api/v1/tones?"));
    }

    /// What TonePush answers for a published tone is recorded under its
    /// stable id, so its next version goes to the same Tone and Song.
    #[test]
    fn a_publish_answer_is_recorded_for_the_next_version() {
        let _scratch = library::tests::Scratch::new("mine-record");
        let (mut app, _events, _cmds) = app();
        let mut tone = details(35, "file-v3", 3, 0);
        tone.summary.version_root_id = Some(34);
        app.record_publish("series-slap", "local-v3", &tone);
        let kept = library::published();
        let record = kept.get("series-slap").expect("recorded");
        assert_eq!((record.tone_id, record.song_id), (34, 12));
        assert_eq!(record.hash, "local-v3");
        assert_eq!(app.mine_count(), 1);
    }
}
