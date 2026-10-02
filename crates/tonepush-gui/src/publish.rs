//! Publishing on TonePush (docs/design/library-workflow-2026-10-02, 14 and
//! flow G): the sheet that asks first and says what a publish does, the queue
//! that publishes several tones one after another, and the record of what was
//! published, so a new version goes to the Tone and Song it already has
//! instead of starting a new Song each time.

use egui::{Pos2, Ui, Vec2};

use crate::cloud;
use crate::library;
use crate::library_view::marker;
use crate::theme::{self, Icon};
use crate::App;

/// What the sheet is asked about.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Asked {
    /// Library tones, by hash: each becomes its tone's next version on
    /// TonePush, or a new Tone.
    Tones(Vec<String>),
    /// One of your tones under another name on TonePush, the name being
    /// typed: renamed by the server where it can, else its file, which the
    /// library holds, published again.
    Rename {
        tone_id: i64,
        hash: Option<String>,
        draft: String,
    },
    /// An earlier version made the one people get: that version's file,
    /// which the library holds, published again.
    Current { hash: String, version: u32 },
    /// A setlist of the library's, by its place in the list, as a list of
    /// your published tones.
    Setlist(usize),
}

/// What the sheet says, and why it cannot go ahead yet when it cannot.
pub(crate) struct Sheet {
    pub title: String,
    pub body: String,
    pub button: String,
    pub tones: Vec<String>,
    pub blocked: Option<String>,
}

/// One tone waiting to be published, and the name it goes under when that is
/// not the library's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Queued {
    pub hash: String,
    pub name: Option<String>,
    /// Kept for you alone once it is up, where the server can.
    pub visibility: Option<cloud::Visibility>,
}

impl Queued {
    pub(crate) fn tone(hash: String) -> Queued {
        Queued {
            hash,
            name: None,
            visibility: None,
        }
    }
}

/// What a tone is on TonePush, as far as this library knows: the record it
/// kept when it published, and TonePush's last answer for it.
pub(crate) struct OnTonePush {
    pub details: Option<cloud::ToneDetails>,
}

impl OnTonePush {
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

    /// How often it was downloaded.
    pub(crate) fn downloads(&self) -> Option<u64> {
        self.details.as_ref().map(|details| {
            details
                .summary
                .downloads_count
                .unwrap_or(details.summary.installs_count)
        })
    }
}

/// Publish a tone, off the UI thread. A tone this library published before
/// goes to the Song it has, as its Tone's next version; when TonePush no
/// longer has that Tone, it starts over as a new one.
pub(crate) fn publish_with(
    client: &cloud::CloudClient,
    token: &str,
    request: &cloud::PublishRequest,
    existing: Option<(i64, i64)>,
) -> Result<cloud::ToneDetails, cloud::PublishError> {
    if let Some((tone_id, song_id)) = existing {
        let gone = matches!(
            client.tone_as(Some(token), tone_id),
            Err(cloud::ApiError::NotFound(_))
        );
        if !gone {
            let mut onto = request.clone();
            onto.song = cloud::PublishSong::Existing(song_id);
            return client.publish(token, &onto);
        }
    }
    client.publish(token, request)
}

/// Publish a tone and, when it is for you alone, make sure TonePush keeps
/// it so: an upload that did not take the choice is hidden right after.
pub(crate) fn publish_hidden(
    client: &cloud::CloudClient,
    token: &str,
    request: &cloud::PublishRequest,
    existing: Option<(i64, i64)>,
    visibility: Option<cloud::Visibility>,
) -> Result<cloud::ToneDetails, cloud::PublishError> {
    let tone = publish_with(client, token, request, existing)?;
    let Some(visibility) = visibility else {
        return Ok(tone);
    };
    if tone.summary.visibility.as_deref() == Some("only_you") {
        return Ok(tone);
    }
    let changes = cloud::ToneChanges {
        visibility: Some(visibility),
        ..Default::default()
    };
    match client.update_tone(token, stable_id(&tone.summary), &changes) {
        Ok(hidden) => Ok(hidden),
        Err(why) => Err(cloud::PublishError::CreatingTone {
            song_id: tone.summary.song_id,
            created_song: None,
            reason: format!(
                "it is published, but TonePush did not keep it for you alone ({why}); hide it \
                 from Mine"
            ),
        }),
    }
}

/// The stable Tone a publish answered with: the root its versions share.
pub(crate) fn stable_id(tone: &cloud::ToneSummary) -> i64 {
    tone.version_root_id.unwrap_or(tone.id)
}

impl App {
    /// Ask before publishing library tones: the sheet says what each
    /// becomes.
    pub(crate) fn ask_to_publish(&mut self, hashes: Vec<String>) {
        let hashes: Vec<String> = hashes
            .into_iter()
            .filter(|hash| self.library_row_of(hash).is_some())
            .collect();
        if hashes.is_empty() {
            return self.note("that tone is no longer in the library".to_owned());
        }
        self.publish_ask = Some(Asked::Tones(hashes));
    }

    /// Ask for a published tone's new name on TonePush.
    pub(crate) fn ask_to_rename_on_tonepush(&mut self, tone_id: i64, hash: String, name: String) {
        self.publish_ask = Some(Asked::Rename {
            tone_id,
            hash: Some(hash),
            draft: name,
        });
    }

    /// Ask before making an earlier version the one people get.
    pub(crate) fn ask_to_make_current(&mut self, hash: String, version: u32) {
        self.publish_ask = Some(Asked::Current { hash, version });
    }

    /// What this library knows of a tone on TonePush, by any of its
    /// revisions.
    pub(crate) fn on_tonepush(&self, hash: &str) -> Option<OnTonePush> {
        let series = self
            .lib_entries
            .iter()
            .find(|entry| entry.hash == hash)
            .map(|entry| entry.series.clone())
            .or_else(|| library::series_of(hash))?;
        let record = self.published.get(&series)?.clone();
        let details = self
            .mine
            .details
            .get(&record.tone_id)
            .and_then(|answer| answer.as_ref().ok())
            .cloned();
        Some(OnTonePush { details })
    }

    /// Start publishing one queued tone: the next version of its Tone when
    /// this library published it before, else a new Tone under its Song.
    pub(crate) fn start_queued(&mut self, queued: Queued, ctx: &egui::Context) {
        if !crate::account::REACHES_TONEPUSH {
            self.publish_queue.clear();
            return;
        }
        let Some(token) = self.config.token.clone() else {
            self.publish_queue.clear();
            return self.problem("sign in first, and then the cloud will publish".into());
        };
        if let Some(publishing) = &self.publishing {
            return self.problem(format!("{} is already being published", publishing.name));
        }
        let Some(row) = self.library_row_of(&queued.hash) else {
            return self.note("that tone is no longer in the library".to_owned());
        };
        let revision = (self.lib_entries[row].hash != queued.hash).then_some(queued.hash.as_str());
        let (hash, name, request) =
            match self.publish_revision(row, revision, queued.name.as_deref()) {
                Ok(built) => built,
                Err(why) => {
                    self.publish_queue.clear();
                    return self.problem(why);
                }
            };
        let series = self.lib_entries[row].series.clone();
        let existing = self
            .published
            .get(&series)
            .map(|record| (record.tone_id, record.song_id));
        let mut request = request;
        let hidden = queued.visibility;
        if hidden == Some(cloud::Visibility::OnlyYou) {
            request.tone.tone.visibility = Some("only_you".to_owned());
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = cloud::CloudClient::new(cloud::site());
            let _ = tx.send(publish_hidden(&client, &token, &request, existing, hidden));
            ctx.request_repaint();
        });
        self.status.clear();
        self.publishing = Some(crate::PublishingJob {
            hash,
            name,
            series,
            answer: rx,
        });
    }

    /// Keep what TonePush answered: which Tone and Song the tone is, so the
    /// next version goes there, and the answer itself for Mine.
    pub(crate) fn record_publish(&mut self, series: &str, hash: &str, tone: &cloud::ToneDetails) {
        let record = library::Published {
            tone_id: stable_id(&tone.summary),
            song_id: tone.summary.song_id,
            hash: hash.to_owned(),
            name: tone.summary.name.clone(),
            at: jiff::Timestamp::now().to_string(),
        };
        if let Err(why) = library::record_published(series, record.clone()) {
            self.note(why);
        }
        self.mine.details.insert(record.tone_id, Ok(tone.clone()));
        self.published.insert(series.to_owned(), record);
    }

    /// The sheet: what publishing does, for whom and as whom, asked once.
    pub(crate) fn publish_window(&mut self, ctx: &egui::Context) {
        let Some(asked) = self.publish_ask.clone() else {
            return;
        };
        if let Asked::Setlist(_) = asked {
            // A setlist names published tones: TonePush's list of them is read
            // before the sheet can say which are there.
            self.list_account(ctx, false);
        }
        let signed_in = self.config.token.is_some();
        let waiting = self.signing_in.as_ref().map(|signing| signing.code.clone());
        let sheet = self.sheet_words(&asked);
        let mut decided = None;
        let mut draft = match &asked {
            Asked::Rename { draft, .. } => Some(draft.clone()),
            _ => None,
        };
        // Only you, where the server can keep a tone for you alone.
        let hides = match &asked {
            Asked::Setlist(_) => self.account.keeps_setlists(),
            Asked::Rename { .. } => false,
            _ => self.account.lists_tones(),
        };
        let mut visibility = self.publish_visibility;
        let (_, close) = theme::dialog(ctx, "publish-sheet", 520.0, |ui| {
            theme::dialog_header(ui, &sheet.title, Some(&sheet.body));
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 24,
                    right: 24,
                    top: 14,
                    bottom: 18,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 10.0;
                    if let Some(draft) = draft.as_mut() {
                        sheet_row(ui, "New name", |ui| {
                            let field = ui.add(
                                egui::TextEdit::singleline(draft)
                                    .id_salt("publish-rename")
                                    .desired_width(260.0)
                                    .font(theme::regular(theme::BODY)),
                            );
                            if !field.has_focus() && !field.lost_focus() {
                                field.request_focus();
                            }
                        });
                    }
                    match (&asked, sheet.tones.as_slice()) {
                        (Asked::Setlist(index), _) => self.sheet_setlist(ui, *index),
                        (_, [one]) => self.sheet_tone(ui, one),
                        (_, several) => {
                            sheet_row(ui, "Tones", |ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 6.0;
                                    for hash in several {
                                        self.sheet_tone_line(ui, hash);
                                    }
                                });
                            });
                        }
                    }
                    if !matches!(asked, Asked::Rename { .. }) {
                        sheet_row(ui, "Who can see it", |ui| {
                            ui.spacing_mut().item_spacing.x = 7.0;
                            if hides {
                                let segments = [
                                    theme::Segment::new("Everyone").icon(Icon::Globe),
                                    theme::Segment::new("Only you").icon(Icon::EyeOff),
                                ];
                                let chosen = match visibility {
                                    cloud::Visibility::Everyone => 0,
                                    cloud::Visibility::OnlyYou => 1,
                                };
                                if let Some(index) = theme::segmented(
                                    ui,
                                    "publish-visibility",
                                    &segments,
                                    Some(chosen),
                                    false,
                                )
                                .clicked
                                {
                                    visibility = if index == 0 {
                                        cloud::Visibility::Everyone
                                    } else {
                                        cloud::Visibility::OnlyYou
                                    };
                                }
                            } else {
                                glyph(ui, Icon::Globe, 14.0, theme::text_soft());
                                theme::label(
                                    ui,
                                    "Everyone, on tonepush.rocks",
                                    theme::regular(theme::BODY),
                                    theme::text(),
                                );
                            }
                        });
                    }
                    sheet_row(ui, "Published as", |ui| match &self.config.account {
                        Some(account) if signed_in => {
                            theme::label(ui, account, theme::regular(theme::BODY), theme::text());
                        }
                        _ => {
                            theme::label(
                                ui,
                                "Not signed in yet",
                                theme::regular(theme::BODY),
                                theme::muted(),
                            );
                        }
                    });
                    if !matches!(asked, Asked::Rename { .. }) {
                        ui.add_space(2.0);
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            glyph(ui, Icon::Info, 14.0, theme::muted());
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(if matches!(asked, Asked::Setlist(_)) {
                                        "Its name, venue and date are the library's; change them \
                                         on its page first."
                                    } else {
                                        "The song and tone details are the library's; change \
                                         them in the tone's details first."
                                    })
                                    .font(theme::regular(12.5))
                                    .color(theme::muted()),
                                )
                                .wrap(),
                            );
                        });
                    }
                });
            let note = match (&waiting, signed_in, &sheet.blocked) {
                (Some(code), _, _) => format!("Approve code {code} on tonepush.rocks to go on."),
                (None, false, _) => {
                    "Signing in opens tonepush.rocks with a code to approve.".to_owned()
                }
                (None, true, Some(why)) => why.clone(),
                _ => String::new(),
            };
            theme::dialog_footer(ui, &note, |ui| {
                if signed_in {
                    if theme::Button::new(&sheet.button)
                        .primary()
                        .icon(Icon::CloudUpload)
                        .hint("Enter")
                        .enabled(sheet.blocked.is_none())
                        .show(ui)
                        .clicked()
                    {
                        decided = Some(true);
                    }
                } else if waiting.is_some() {
                    let _ = theme::Button::new("Waiting for approval")
                        .primary()
                        .enabled(false)
                        .show(ui);
                } else if theme::Button::new("Sign in to publish")
                    .primary()
                    .icon(Icon::User)
                    .show(ui)
                    .clicked()
                {
                    self.start_signing_in(ui.ctx());
                }
                if theme::Button::new("Cancel").show(ui).clicked() {
                    decided = Some(false);
                }
            });
        });
        self.publish_visibility = visibility;
        if let (Some(draft), Some(Asked::Rename { draft: kept, .. })) =
            (draft.as_ref(), self.publish_ask.as_mut())
        {
            kept.clone_from(draft);
        }
        if signed_in
            && sheet.blocked.is_none()
            && decided.is_none()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            decided = Some(true);
        }
        if close && decided.is_none() {
            decided = Some(false);
        }
        match decided {
            Some(true) => {
                let Some(asked) = self.publish_ask.take() else {
                    return;
                };
                let hidden = (hides && visibility == cloud::Visibility::OnlyYou)
                    .then_some(cloud::Visibility::OnlyYou);
                self.publish_visibility = cloud::Visibility::Everyone;
                let queued: Vec<Queued> = match asked {
                    Asked::Tones(hashes) => hashes
                        .into_iter()
                        .map(|hash| Queued {
                            hash,
                            name: None,
                            visibility: hidden,
                        })
                        .collect(),
                    Asked::Rename {
                        tone_id,
                        hash,
                        draft,
                    } => {
                        let name = draft.trim().to_owned();
                        if name.is_empty() {
                            return self.note("a tone needs a name".to_owned());
                        }
                        match hash {
                            // The server renames it without a new upload.
                            _ if self.account.lists_tones() => {
                                self.rename_tone_on_tonepush(tone_id, name, ctx);
                                return;
                            }
                            Some(hash) => vec![Queued {
                                hash,
                                name: Some(name),
                                visibility: None,
                            }],
                            None => return,
                        }
                    }
                    Asked::Current { hash, .. } => vec![Queued::tone(hash)],
                    Asked::Setlist(index) => {
                        let Some((_, setlist)) = self.lib_setlists.get(index).cloned() else {
                            return;
                        };
                        match self.setlist_for_tonepush(&setlist) {
                            Ok(mut new) => {
                                new.visibility = hidden;
                                self.create_setlist_on_tonepush(new, ctx);
                            }
                            Err(missing) => self.note(format!(
                                "not on TonePush yet: {}",
                                crate::put::listed(&missing)
                            )),
                        }
                        return;
                    }
                };
                self.publish_queue.extend(queued);
                if self.publishing.is_none() {
                    if let Some(first) = self.publish_queue.pop_front() {
                        self.start_queued(first, ctx);
                    }
                }
            }
            Some(false) => {
                self.publish_ask = None;
                self.publish_visibility = cloud::Visibility::Everyone;
            }
            None => {}
        }
    }

    /// A setlist's rows on the sheet: what it is, its presets and which of
    /// them TonePush does not have yet, and the pedal it is for.
    fn sheet_setlist(&self, ui: &mut Ui, index: usize) {
        let Some((_, setlist)) = self.lib_setlists.get(index) else {
            return;
        };
        let what = [
            setlist.name.as_str(),
            setlist.venue.trim(),
            setlist.date.trim(),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        sheet_row(ui, "Setlist", |ui| {
            theme::label(ui, &what, theme::regular(theme::BODY), theme::text());
        });
        let filled = setlist.filled();
        sheet_row(ui, "Presets", |ui| {
            let (words, ink) = match (&self.account.tones, self.setlist_for_tonepush(setlist)) {
                (None, _) => ("Reading your tones on TonePush…".to_owned(), theme::muted()),
                (Some(_), Ok(_)) => (
                    format!("{filled}, each one of your tones on TonePush"),
                    theme::text(),
                ),
                (Some(_), Err(missing)) => {
                    let shown: Vec<String> = missing.iter().take(3).cloned().collect();
                    let more = missing.len().saturating_sub(shown.len());
                    let list = if more > 0 {
                        format!("{} and {more} more", shown.join(", "))
                    } else {
                        crate::put::listed(&shown)
                    };
                    (format!("Not on TonePush yet: {list}"), theme::hot())
                }
            };
            ui.add(
                egui::Label::new(
                    egui::RichText::new(words)
                        .font(theme::regular(theme::BODY))
                        .color(ink),
                )
                .wrap(),
            );
        });
        sheet_row(ui, "For", |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let found = self.setlist_marker(setlist);
            marker(ui, found.as_ref(), true);
        });
    }

    /// The sheet's words, its button, the tones it is about, and why it
    /// cannot go ahead yet.
    fn sheet_words(&self, asked: &Asked) -> Sheet {
        let (title, body, button, tones) = match asked {
            Asked::Tones(hashes) if hashes.len() > 1 => (
                format!("Publish {} tones on TonePush?", hashes.len()),
                "One after another, each as its tone's next version or a new Tone, stopping \
                 at the first TonePush refuses."
                    .to_owned(),
                format!("Publish {}", hashes.len()),
                hashes.clone(),
            ),
            Asked::Tones(hashes) => {
                let hash = hashes.first().cloned().unwrap_or_default();
                let name = self.tone_name(&hash);
                match self.on_tonepush(&hash).and_then(|on| {
                    let (current, versions) = on.versions()?;
                    Some((current, versions, on.downloads().unwrap_or(0)))
                }) {
                    Some((current, versions, downloads)) => {
                        let next = versions + 1;
                        (
                            format!("Publish {name} v{next} on TonePush?"),
                            format!(
                                "TonePush has v{current} of this tone, downloaded {}. v{next} \
                                 becomes the version people get; v{current} stays on its page \
                                 and can still be downloaded.",
                                times(downloads)
                            ),
                            format!("Publish v{next}"),
                            vec![hash],
                        )
                    }
                    None if self.on_tonepush(&hash).is_some() => (
                        format!("Publish a new version of {name}?"),
                        "It becomes the version people get on TonePush; the versions before \
                         it stay on its page."
                            .to_owned(),
                        "Publish".to_owned(),
                        vec![hash],
                    ),
                    None => (
                        format!("Publish {name} on TonePush?"),
                        "It goes on tonepush.rocks as a new Tone under its Song, for anyone \
                         with the pedal to find and download."
                            .to_owned(),
                        "Publish".to_owned(),
                        vec![hash],
                    ),
                }
            }
            Asked::Rename { tone_id, hash, .. } => {
                let name = self
                    .mine_row(*tone_id)
                    .map(|row| row.name())
                    .filter(|name| !name.is_empty())
                    .or_else(|| hash.as_deref().map(|hash| self.tone_name(hash)))
                    .unwrap_or_else(|| "this tone".to_owned());
                let body = if self.account.lists_tones() {
                    "Only its name changes on tonepush.rocks: its file, versions and downloads \
                     stay, and the library keeps its own name for it."
                } else {
                    "TonePush cannot rename a tone on its own yet, so its file is published \
                     again under the new name. Its page, downloads and versions stay; the \
                     library keeps its own name for it."
                };
                (
                    format!("Rename {name} on TonePush?"),
                    body.to_owned(),
                    "Rename".to_owned(),
                    hash.iter().cloned().collect(),
                )
            }
            Asked::Current { hash, version } => {
                let name = self.tone_name(hash);
                (
                    format!("Make v{version} of {name} the version people get?"),
                    format!(
                        "Its file is published again, and TonePush makes v{version} current \
                         without adding a version. The others stay on its page."
                    ),
                    format!("Make v{version} current"),
                    vec![hash.clone()],
                )
            }
            Asked::Setlist(index) => {
                let name = self
                    .lib_setlists
                    .get(*index)
                    .map(|(_, setlist)| setlist.name.clone())
                    .unwrap_or_default();
                (
                    format!("Publish {name} on TonePush?"),
                    "On TonePush a setlist is a list of your published tones, in the order the \
                     pedal plays them, each the version this setlist holds."
                        .to_owned(),
                    "Publish setlist".to_owned(),
                    Vec::new(),
                )
            }
        };
        let blocked = match asked {
            Asked::Setlist(index) => match (&self.account.tones, self.lib_setlists.get(*index)) {
                (None, _) => Some("Reading your tones on TonePush…".to_owned()),
                (Some(_), Some((_, setlist))) => self
                    .setlist_for_tonepush(setlist)
                    .err()
                    .map(|_| "Publish those tones first, then the setlist.".to_owned()),
                (Some(_), None) => Some("That setlist is no longer in the library".to_owned()),
            },
            _ => None,
        };
        Sheet {
            title,
            body,
            button,
            tones,
            blocked,
        }
    }

    /// A library tone's name, by any of its revisions.
    fn tone_name(&self, hash: &str) -> String {
        self.library_row_of(hash)
            .map(|row| self.lib_entries[row].name.clone())
            .or_else(|| library::meta_of(hash).map(|meta| meta.name))
            .unwrap_or_else(|| "this tone".to_owned())
    }

    /// One tone's rows on the sheet: its Song, the Tone, and what it is for.
    fn sheet_tone(&self, ui: &mut Ui, hash: &str) {
        let Some(entry) = self.library_row_of(hash).map(|row| &self.lib_entries[row]) else {
            return;
        };
        let song = if entry.meta.song.trim().is_empty() {
            format!("{}, an original", entry.name)
        } else if entry.meta.artist.trim().is_empty() {
            entry.meta.song.trim().to_owned()
        } else {
            format!("{} · {}", entry.meta.song.trim(), entry.meta.artist.trim())
        };
        sheet_row(ui, "Song", |ui| {
            theme::label(ui, &song, theme::regular(theme::BODY), theme::text());
        });
        let tone = [
            entry.name.as_str(),
            entry.meta.character.trim(),
            entry.meta.part.trim(),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .map(capitalised)
        .collect::<Vec<_>>()
        .join(" · ");
        sheet_row(ui, "Tone", |ui| {
            theme::label(ui, &tone, theme::regular(theme::BODY), theme::text());
        });
        sheet_row(ui, "For", |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            marker(ui, entry.marker.as_ref(), true);
            let firmware = entry.firmware.trim();
            if !firmware.is_empty() {
                theme::label(
                    ui,
                    &format!("firmware {firmware}"),
                    theme::regular(theme::BODY),
                    theme::text(),
                );
            }
        });
    }

    /// One of several tones on the sheet: its marker, its name, and whether
    /// it is a new Tone or its next version.
    fn sheet_tone_line(&self, ui: &mut Ui, hash: &str) {
        let Some(entry) = self.library_row_of(hash).map(|row| &self.lib_entries[row]) else {
            return;
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            marker(ui, entry.marker.as_ref(), true);
            theme::label(ui, &entry.name, theme::semibold(theme::BODY), theme::text());
            let what = match self.on_tonepush(hash).and_then(|on| on.versions()) {
                Some((_, versions)) => format!("v{} on TonePush", versions + 1),
                None if self.on_tonepush(hash).is_some() => "its next version".to_owned(),
                None => "a new Tone".to_owned(),
            };
            theme::label(ui, &what, theme::regular(12.5), theme::muted());
        });
    }
}

/// "312 times", "once".
fn times(count: u64) -> String {
    match count {
        0 => "no times yet".to_owned(),
        1 => "once".to_owned(),
        count => format!("{} times", crate::format_count(count)),
    }
}

/// "clean" as the sheet writes it: "Clean".
fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// One of the sheet's rows: what it is, in the quiet voice, then the value.
fn sheet_row(ui: &mut Ui, label: &str, value: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(122.0, 20.0), egui::Sense::hover());
        let galley = crate::shell::galley(ui, label, theme::regular(theme::BODY), theme::muted());
        crate::shell::paint_line(ui, galley, rect.left(), rect.center().y);
        value(ui);
    });
}

/// An icon in a row of words, as tall as they are.
fn glyph(ui: &mut Ui, icon: Icon, size: f32, colour: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size + 2.0), egui::Sense::hover());
    theme::paint_icon(
        ui,
        icon,
        Pos2::new(rect.center().x, rect.center().y),
        size,
        colour,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A TonePush that does not answer is not taken to have lost the Tone:
    /// the next version still goes to the Song it has.
    #[test]
    fn a_tonepush_that_does_not_answer_is_not_taken_for_a_gone_tone() {
        let request = cloud::PublishRequest {
            song: cloud::PublishSong::New(cloud::CreateSongRequest {
                creator_name: "Noa Calder".to_owned(),
                song: cloud::NewSong {
                    title: "Slapback Twang".to_owned(),
                    kind: cloud::SongKind::Original,
                    artist_name: None,
                    description: None,
                    tags: Vec::new(),
                    genre_ids: Vec::new(),
                },
            }),
            tone: cloud::CreateToneRequest::default(),
        };
        // No server answers here: the check cannot say the tone is gone, so
        // the publish goes to the Song it has, and fails to arrive.
        let client = cloud::CloudClient::new("http://127.0.0.1:9");
        let answer = publish_with(&client, "token", &request, Some((34, 12)));
        match answer {
            Err(cloud::PublishError::CreatingTone {
                song_id: 12,
                created_song: None,
                ..
            }) => {}
            other => panic!("expected the existing Song, got {other:?}"),
        }
    }

    fn new_song_request() -> cloud::PublishRequest {
        cloud::PublishRequest {
            song: cloud::PublishSong::New(cloud::CreateSongRequest {
                creator_name: "Noa Calder".to_owned(),
                song: cloud::NewSong {
                    title: "Slapback Twang".to_owned(),
                    kind: cloud::SongKind::Original,
                    artist_name: None,
                    description: None,
                    tags: Vec::new(),
                    genre_ids: Vec::new(),
                },
            }),
            tone: cloud::CreateToneRequest {
                creator_name: "Noa Calder".to_owned(),
                tone: cloud::NewTone {
                    name: "Slapback Twang".to_owned(),
                    series_id: Some("series-slapback".to_owned()),
                    ..Default::default()
                },
            },
        }
    }

    fn first_line(request: &[u8]) -> String {
        String::from_utf8_lossy(request)
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    /// A mocked TonePush that still has the Tone: the next version goes to
    /// its Song, and no new Song is made.
    #[test]
    fn a_new_version_goes_to_the_song_the_tone_has() {
        use crate::cloud::tests::{tone_json, StubServer};
        let server = StubServer::start(vec![
            (200, tone_json(34, 12, "Slapback Twang")),
            (201, tone_json(35, 12, "Slapback Twang")),
        ]);
        let client = cloud::CloudClient::new(&server.base);
        let answer = publish_with(&client, "token", &new_song_request(), Some((34, 12)));
        assert_eq!(answer.map(|tone| tone.summary.song_id), Ok(12));
        let requests = server.finish();
        assert!(first_line(&requests[0]).starts_with("GET /api/v1/tones/34 "));
        assert!(first_line(&requests[1]).starts_with("POST /api/v1/songs/12/tones "));
    }

    /// A mocked TonePush that no longer has the Tone: it starts over, Song
    /// first.
    #[test]
    fn a_tone_gone_from_tonepush_starts_over() {
        use crate::cloud::tests::{song_json, tone_json, StubServer};
        let mut song = song_json(50, "Slapback Twang");
        song["tone_count"] = 0.into();
        song["tones"] = serde_json::json!([]);
        let server = StubServer::start(vec![
            (404, serde_json::json!({"error": "Not found"})),
            (201, song),
            (201, tone_json(60, 50, "Slapback Twang")),
        ]);
        let client = cloud::CloudClient::new(&server.base);
        let answer = publish_with(&client, "token", &new_song_request(), Some((34, 12)));
        assert_eq!(answer.map(|tone| tone.summary.song_id), Ok(50));
        let requests = server.finish();
        assert!(first_line(&requests[1]).starts_with("POST /api/v1/songs "));
        assert!(first_line(&requests[2]).starts_with("POST /api/v1/songs/50/tones "));
    }

    /// What the sheet says of a tone TonePush already has: the version it
    /// becomes, and the one that stays.
    #[test]
    fn the_sheet_counts_the_version_a_publish_becomes() {
        let on = OnTonePush {
            details: Some({
                let mut tone: cloud::ToneDetails =
                    serde_json::from_str(include_str!("../tests/fixtures/cloud/tone-details.json"))
                        .unwrap();
                tone.summary.version_number = Some(2);
                tone.summary.versions_count = 2;
                tone.summary.downloads_count = Some(312);
                tone
            }),
        };
        assert_eq!(on.versions(), Some((2, 2)));
        assert_eq!(on.downloads(), Some(312));
        assert_eq!(times(312), "312 times");
        assert_eq!(times(1), "once");
        assert_eq!(capitalised("clean"), "Clean");
    }
}
