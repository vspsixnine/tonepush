//! Your account on TonePush, where the server can say more than the public
//! catalog (docs/design/library-workflow-2026-10-02, 19 and 20, and the
//! server items S1 to S7): everything you published from any computer, tones
//! and setlists; who can see each one; deleting one; your page.
//!
//! Each of these is asked of the server only once a probe has found it
//! offers the endpoint. An older deployment, or a `TONEPUSH_SITE` override,
//! may not: then Mine keeps to what this library published (`mine.rs`), and
//! the items that need the server are left out of the menus, never failing on
//! a click.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::cloud;
use crate::library;
use crate::theme::{self, Icon};
use crate::App;

/// How long the account's lists are kept before Mine asks again.
const FRESH_FOR: Duration = Duration::from_secs(120);

/// Whether the app itself may reach TonePush. A test build never does: its
/// configuration may be the person's own, token and all, and tests and
/// screenshots answer with mocked responses of their own instead.
pub(crate) const REACHES_TONEPUSH: bool = !cfg!(test);

/// The account's lists, as one answer.
pub(crate) struct Listed {
    pub me: Option<cloud::Me>,
    pub tones: Vec<cloud::ToneDetails>,
    pub setlists: Option<Vec<cloud::SetlistSummary>>,
}

/// What a change made on TonePush answered.
pub(crate) enum Changed {
    Tone(Result<Box<cloud::ToneDetails>, cloud::ApiError>),
    ToneDeleted(i64, Result<(), cloud::ApiError>),
    Setlist(Result<Box<cloud::SetlistDetails>, cloud::ApiError>),
    SetlistDeleted(i64, Result<(), cloud::ApiError>),
}

/// Something of yours on TonePush that a question is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Yours {
    Tone(i64),
    Setlist(i64),
}

/// The account, as far as the server has answered.
#[derive(Default)]
pub(crate) struct Account {
    /// Which signed-in endpoints the server offers; `None` until asked.
    pub caps: Option<cloud::Capabilities>,
    probing: Option<Receiver<cloud::Capabilities>>,
    /// Whether the probe was made signed in, so signing in asks again.
    probed_signed_in: bool,
    pub me: Option<cloud::Me>,
    /// Every tone you published, from any computer, in any state.
    pub tones: Option<Vec<cloud::ToneDetails>>,
    /// Your setlists.
    pub setlists: Option<Vec<cloud::SetlistSummary>>,
    listing: Option<Receiver<Result<Listed, cloud::ApiError>>>,
    listed: Option<Instant>,
    /// Why the lists could not be read.
    pub problem: Option<String>,
    changing: Vec<Receiver<Changed>>,
    /// Waiting on an answer about deleting it from TonePush.
    pub confirm_delete: Option<Yours>,
}

impl Account {
    /// Whether the server lists your account's tones.
    pub(crate) fn lists_tones(&self) -> bool {
        self.caps.is_some_and(|caps| caps.account)
    }

    /// Whether the server keeps setlists.
    pub(crate) fn keeps_setlists(&self) -> bool {
        self.caps.is_some_and(|caps| caps.setlists)
    }

    /// Whether something is still on its way.
    pub(crate) fn busy(&self) -> bool {
        self.listing.is_some() || !self.changing.is_empty()
    }

    /// Hold a screenshot's answers as fresh and the server as asked.
    #[cfg(test)]
    pub(crate) fn mark_listed(&mut self, caps: cloud::Capabilities) {
        self.caps = Some(caps);
        self.probed_signed_in = true;
        self.listed = Some(Instant::now());
    }
}

/// Read the account's lists off the UI thread.
pub(crate) fn list_with(
    client: &cloud::CloudClient,
    token: &str,
    caps: cloud::Capabilities,
) -> Result<Listed, cloud::ApiError> {
    let me = client.me(token).ok();
    let tones = client.my_tones(token)?;
    let setlists = if caps.setlists {
        client.my_setlists(token).ok()
    } else {
        None
    };
    Ok(Listed {
        me,
        tones,
        setlists,
    })
}

impl App {
    /// Ask once which of the signed-in endpoints the server offers, and
    /// again after signing in.
    pub(crate) fn probe_account(&mut self, ctx: &egui::Context) {
        if !REACHES_TONEPUSH {
            return;
        }
        let signed_in = self.config.token.is_some();
        if self.account.probing.is_some()
            || (self.account.caps.is_some() && self.account.probed_signed_in == signed_in)
        {
            return;
        }
        let token = self.config.token.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = cloud::CloudClient::new(cloud::site());
            let _ = tx.send(client.probe(token.as_deref()));
            ctx.request_repaint();
        });
        self.account.probing = Some(rx);
        self.account.probed_signed_in = signed_in;
    }

    /// Ask for the account's lists, when the server offers them and they are
    /// old or never read.
    pub(crate) fn list_account(&mut self, ctx: &egui::Context, now: bool) {
        if !REACHES_TONEPUSH {
            return;
        }
        let Some(caps) = self.account.caps.filter(|caps| caps.account) else {
            return;
        };
        let Some(token) = self.config.token.clone() else {
            return;
        };
        if self.account.listing.is_some()
            || (!now
                && self
                    .account
                    .listed
                    .is_some_and(|listed| listed.elapsed() < FRESH_FOR))
        {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = cloud::CloudClient::new(cloud::site());
            let _ = tx.send(list_with(&client, &token, caps));
            ctx.request_repaint();
        });
        self.account.listing = Some(rx);
        self.account.listed = Some(Instant::now());
    }

    /// Collect what the server answered: the probe, the lists, and changes.
    pub(crate) fn settle_account(&mut self) {
        if let Some(caps) = self
            .account
            .probing
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.account.probing = None;
            self.account.caps = Some(caps);
            self.account.listed = None;
        }
        if let Some(answer) = self
            .account
            .listing
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.account.listing = None;
            match answer {
                Ok(listed) => {
                    if let Some(page) = listed.me.as_ref().and_then(|me| me.profile_url.clone()) {
                        if self.config.profile_url.as_deref() != Some(page.as_str()) {
                            self.config.profile_url = Some(page);
                            self.config.save();
                        }
                    }
                    self.account.me = listed.me;
                    self.account.tones = Some(listed.tones);
                    self.account.setlists = listed.setlists;
                    self.account.problem = None;
                }
                Err(cloud::ApiError::SignedOut) => {
                    self.account.tones = None;
                    self.account.setlists = None;
                    self.account.problem = Some(cloud::ApiError::SignedOut.to_string());
                    self.config.sign_out();
                }
                Err(cloud::ApiError::Unavailable) => {
                    // An older server after all: Mine keeps to this library.
                    self.account.caps = Some(cloud::Capabilities::default());
                }
                Err(other) => self.account.problem = Some(other.to_string()),
            }
        }
        let mut answers = Vec::new();
        self.account.changing.retain(|rx| match rx.try_recv() {
            Ok(answer) => {
                answers.push(answer);
                false
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
        });
        for answer in answers {
            self.account_changed(answer);
        }
    }

    /// What a change answered, put where Mine reads it.
    fn account_changed(&mut self, answer: Changed) {
        match answer {
            Changed::Tone(Ok(tone)) => {
                let tone = *tone;
                let id = crate::publish::stable_id(&tone.summary);
                if let Some(tones) = self.account.tones.as_mut() {
                    match tones
                        .iter_mut()
                        .find(|known| crate::publish::stable_id(&known.summary) == id)
                    {
                        Some(known) => *known = tone.clone(),
                        None => tones.push(tone.clone()),
                    }
                }
                self.mine.details.insert(id, Ok(tone));
            }
            Changed::ToneDeleted(id, Ok(())) => {
                let name = self.mine_row(id).map(|row| row.name()).unwrap_or_default();
                if let Some(tones) = self.account.tones.as_mut() {
                    tones.retain(|tone| crate::publish::stable_id(&tone.summary) != id);
                }
                self.mine.details.remove(&id);
                let series: Vec<String> = self
                    .published
                    .iter()
                    .filter(|(_, record)| record.tone_id == id)
                    .map(|(series, _)| series.clone())
                    .collect();
                for series in series {
                    if let Err(why) = library::forget_published(&series) {
                        self.note(why);
                    }
                    self.published.remove(&series);
                }
                if self.mine.selected == Some(id) {
                    self.mine.selected = None;
                }
                self.note(format!("deleted {name} from TonePush"));
            }
            Changed::Setlist(Ok(setlist)) => {
                let summary = setlist.summary.clone();
                if let Some(setlists) = self.account.setlists.as_mut() {
                    match setlists.iter_mut().find(|known| known.id == summary.id) {
                        Some(known) => *known = summary,
                        None => setlists.insert(0, summary),
                    }
                }
            }
            Changed::SetlistDeleted(id, Ok(())) => {
                let name = self
                    .account
                    .setlists
                    .as_ref()
                    .and_then(|setlists| setlists.iter().find(|known| known.id == id))
                    .map(|known| known.name.clone())
                    .unwrap_or_default();
                if let Some(setlists) = self.account.setlists.as_mut() {
                    setlists.retain(|known| known.id != id);
                }
                if self.mine.selected_setlist == Some(id) {
                    self.mine.selected_setlist = None;
                }
                self.note(format!("deleted {name} from TonePush"));
            }
            Changed::Tone(Err(why))
            | Changed::ToneDeleted(_, Err(why))
            | Changed::Setlist(Err(why))
            | Changed::SetlistDeleted(_, Err(why)) => {
                if why == cloud::ApiError::SignedOut {
                    self.config.sign_out();
                }
                self.problem(format!("TonePush did not take the change: {why}"));
            }
        }
    }

    /// Make a change on TonePush off the UI thread.
    fn change_on_tonepush(
        &mut self,
        ctx: &egui::Context,
        change: impl FnOnce(&cloud::CloudClient, &str) -> Changed + Send + 'static,
    ) {
        if !REACHES_TONEPUSH {
            return;
        }
        let Some(token) = self.config.token.clone() else {
            return self.problem("sign in to TonePush first".to_owned());
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let client = cloud::CloudClient::new(cloud::site());
            let _ = tx.send(change(&client, &token));
            ctx.request_repaint();
        });
        self.account.changing.push(rx);
    }

    /// Show one of your tones to everyone, or keep it for yourself.
    pub(crate) fn set_tone_visibility(
        &mut self,
        id: i64,
        visibility: cloud::Visibility,
        ctx: &egui::Context,
    ) {
        self.change_on_tonepush(ctx, move |client, token| {
            Changed::Tone(
                client
                    .update_tone(
                        token,
                        id,
                        &cloud::ToneChanges {
                            visibility: Some(visibility),
                            ..Default::default()
                        },
                    )
                    .map(Box::new),
            )
        });
    }

    /// Rename one of your tones on TonePush, without publishing it again.
    pub(crate) fn rename_tone_on_tonepush(&mut self, id: i64, name: String, ctx: &egui::Context) {
        self.change_on_tonepush(ctx, move |client, token| {
            Changed::Tone(
                client
                    .update_tone(
                        token,
                        id,
                        &cloud::ToneChanges {
                            name: Some(name),
                            ..Default::default()
                        },
                    )
                    .map(Box::new),
            )
        });
    }

    /// Show one of your setlists to everyone, or keep it for yourself.
    pub(crate) fn set_setlist_visibility(
        &mut self,
        id: i64,
        visibility: cloud::Visibility,
        ctx: &egui::Context,
    ) {
        self.change_on_tonepush(ctx, move |client, token| {
            Changed::Setlist(
                client
                    .update_setlist(
                        token,
                        id,
                        &cloud::SetlistChanges {
                            visibility: Some(visibility),
                            ..Default::default()
                        },
                    )
                    .map(Box::new),
            )
        });
    }

    /// Put a setlist on TonePush.
    pub(crate) fn create_setlist_on_tonepush(
        &mut self,
        setlist: cloud::NewSetlist,
        ctx: &egui::Context,
    ) {
        self.change_on_tonepush(ctx, move |client, token| {
            Changed::Setlist(client.create_setlist(token, &setlist).map(Box::new))
        });
    }

    /// Who can see one of your tones on TonePush.
    pub(crate) fn tone_visibility(&self, id: i64) -> Option<cloud::Visibility> {
        let tone = self
            .account
            .tones
            .as_ref()?
            .iter()
            .find(|tone| crate::publish::stable_id(&tone.summary) == id)?;
        Some(match tone.summary.visibility.as_deref() {
            Some("only_you") => cloud::Visibility::OnlyYou,
            _ => cloud::Visibility::Everyone,
        })
    }

    /// The question before something of yours goes from TonePush, with
    /// what it takes and what stays (sheet 20).
    pub(crate) fn confirm_tonepush_delete_window(&mut self, ctx: &egui::Context) {
        let Some(yours) = self.account.confirm_delete else {
            return;
        };
        let (title, body, facts) = match yours {
            Yours::Tone(id) => {
                let Some(row) = self.mine_row(id) else {
                    self.account.confirm_delete = None;
                    return;
                };
                let versions = row
                    .details
                    .as_ref()
                    .map(|tone| tone.versions.len().max(1))
                    .unwrap_or(1);
                let numbers = row
                    .details
                    .as_ref()
                    .map(|tone| {
                        tone.versions
                            .iter()
                            .map(|version| format!("v{}", version.number))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut facts = vec![
                    (
                        Icon::Remove,
                        versions.to_string(),
                        if versions == 1 {
                            "version deleted".to_owned()
                        } else {
                            format!("versions deleted, {}", crate::put::listed(&numbers))
                        },
                        true,
                    ),
                    (
                        Icon::Download,
                        crate::format_count(row.downloads()),
                        "downloads: whoever kept it keeps their copy".to_owned(),
                        false,
                    ),
                ];
                if let Some(index) = row.local {
                    facts.push((
                        Icon::Computer,
                        format!("v{}", self.lib_entries[index].version),
                        "stays in your library, untouched".to_owned(),
                        false,
                    ));
                }
                let every = match versions {
                    1 => "its version goes",
                    2 => "both versions go",
                    _ => "every version goes",
                };
                (
                    format!("Delete {} from TonePush?", row.name()),
                    format!(
                        "Its page and {every} from tonepush.rocks. This cannot be undone there; \
                         publishing it again starts it over at v1."
                    ),
                    facts,
                )
            }
            Yours::Setlist(id) => {
                let Some(setlist) = self
                    .account
                    .setlists
                    .as_ref()
                    .and_then(|setlists| setlists.iter().find(|known| known.id == id))
                    .cloned()
                else {
                    self.account.confirm_delete = None;
                    return;
                };
                (
                    format!("Delete {} from TonePush?", setlist.name),
                    "Its page goes from tonepush.rocks. The tones in it stay on TonePush, \
                     and your setlist stays in your library."
                        .to_owned(),
                    vec![(
                        Icon::ListMusic,
                        setlist.slot_count.to_string(),
                        "presets: each stays on TonePush as its own tone".to_owned(),
                        false,
                    )],
                )
            }
        };
        let mut decided = None;
        let (_, close) = theme::dialog(ctx, "confirm-tonepush-delete", 474.0, |ui| {
            theme::dialog_header(ui, &title, Some(&body));
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 24,
                    right: 24,
                    top: 14,
                    bottom: 18,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    theme::outcomes(
                        ui,
                        &facts
                            .iter()
                            .map(|(icon, count, words, loud)| theme::Outcome {
                                icon: *icon,
                                count: count.clone(),
                                sentence: words.clone(),
                                quiet: !*loud,
                            })
                            .collect::<Vec<_>>(),
                    );
                });
            let note = match yours {
                Yours::Tone(_) if self.account.lists_tones() => {
                    "Hiding it keeps the page for you alone."
                }
                _ => "",
            };
            theme::dialog_footer(ui, note, |ui| {
                if theme::Button::new("Delete from TonePush")
                    .danger()
                    .show(ui)
                    .clicked()
                {
                    decided = Some(true);
                }
                if theme::Button::new("Cancel").show(ui).clicked() {
                    decided = Some(false);
                }
            });
        });
        if close && decided.is_none() {
            decided = Some(false);
        }
        match decided {
            Some(true) => {
                self.account.confirm_delete = None;
                match yours {
                    Yours::Tone(id) => self.change_on_tonepush(ctx, move |client, token| {
                        Changed::ToneDeleted(id, client.delete_tone(token, id))
                    }),
                    Yours::Setlist(id) => self.change_on_tonepush(ctx, move |client, token| {
                        Changed::SetlistDeleted(id, client.delete_setlist(token, id))
                    }),
                }
            }
            Some(false) => self.account.confirm_delete = None,
            None => {}
        }
    }

    /// A local setlist as TonePush keeps one: your published tones in its
    /// order, each pinned to the version the setlist plays. Answers the
    /// slots that are not on TonePush in that version instead, by label.
    pub(crate) fn setlist_for_tonepush(
        &self,
        setlist: &library::Setlist,
    ) -> Result<cloud::NewSetlist, Vec<String>> {
        let tones = self.account.tones.clone().unwrap_or_default();
        let mut slots = Vec::new();
        let mut missing = Vec::new();
        for (slot, tone) in setlist.slots.iter().enumerate() {
            if tone.is_empty() {
                continue;
            }
            let file = self
                .portable_hashes
                .get(&tone.hash)
                .cloned()
                .or_else(|| library::portable_hash(&tone.hash));
            let found = file.as_deref().and_then(|file| {
                tones.iter().find_map(|published| {
                    let id = crate::publish::stable_id(&published.summary);
                    published
                        .versions
                        .iter()
                        .find(|version| version.file_sha256 == file)
                        .map(|version| (id, Some(version.number)))
                        .or_else(|| {
                            (published.file_sha256.as_deref() == Some(file)).then_some((id, None))
                        })
                })
            });
            match found {
                Some((tone_id, version)) => slots.push(cloud::NewSetlistSlot { tone_id, version }),
                None => missing.push(format!(
                    "{} {}",
                    self.active_slot_label(slot as i64),
                    tone.name
                )),
            }
        }
        if !missing.is_empty() {
            return Err(missing);
        }
        Ok(cloud::NewSetlist {
            name: setlist.name.clone(),
            venue: (!setlist.venue.trim().is_empty()).then(|| setlist.venue.trim().to_owned()),
            performed_on: performed_on(&setlist.date),
            device_name: self
                .setlist_marker(setlist)
                .map(|marker| marker.catalog_device()),
            device_id: None,
            visibility: None,
            slots,
        })
    }
}

/// A setlist's date as TonePush keeps it, YYYY-MM-DD, from the way the
/// library writes it ("4 Oct 2026"), when it can be read.
pub(crate) fn performed_on(date: &str) -> Option<String> {
    let mut parts = date.split_whitespace();
    let day: u32 = parts.next()?.parse().ok()?;
    let month = parts.next()?.to_ascii_lowercase();
    let year: u32 = parts.next()?.parse().ok()?;
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let month = MONTHS.iter().position(|name| month.starts_with(name))? + 1;
    (1..=31)
        .contains(&day)
        .then(|| format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::tests::{tone_json, StubServer};

    /// The account's lists come from the server's own endpoints: who you
    /// are, your tones, and your setlists when it keeps them; a mocked
    /// server answers.
    #[test]
    fn the_account_lists_come_from_its_endpoints() {
        let server = StubServer::start(vec![
            (
                200,
                serde_json::json!({"id": 7, "name": "Noa Calder", "slug": "noa-calder",
                    "profile_url": "https://tonepush.example/noa-calder"}),
            ),
            (
                200,
                serde_json::json!({"tones": [tone_json(34, 12, "Slapback Twang")]}),
            ),
            (200, serde_json::json!({"setlists": []})),
        ]);
        let client = cloud::CloudClient::new(&server.base);
        let listed = list_with(
            &client,
            "token",
            cloud::Capabilities {
                account: true,
                setlists: true,
            },
        )
        .unwrap();
        assert_eq!(
            listed.me.and_then(|me| me.profile_url).as_deref(),
            Some("https://tonepush.example/noa-calder")
        );
        assert_eq!(listed.tones.len(), 1);
        assert_eq!(listed.setlists.map(|setlists| setlists.len()), Some(0));
        let requests = server.finish();
        let lines: Vec<String> = requests
            .iter()
            .map(|request| {
                String::from_utf8_lossy(request)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect();
        assert!(lines[0].starts_with("GET /api/v1/me "));
        assert!(lines[1].starts_with("GET /api/v1/me/tones "));
        assert!(lines[2].starts_with("GET /api/v1/setlists "));
        assert!(String::from_utf8_lossy(&requests[1]).contains("Bearer token"));
    }

    /// A server without setlists is not asked for them.
    #[test]
    fn a_server_without_setlists_is_not_asked_for_them() {
        let server = StubServer::start(vec![
            (404, serde_json::json!({"error": "Not found"})),
            (200, serde_json::json!({"tones": []})),
        ]);
        let client = cloud::CloudClient::new(&server.base);
        let listed = list_with(
            &client,
            "token",
            cloud::Capabilities {
                account: true,
                setlists: false,
            },
        )
        .unwrap();
        assert!(listed.me.is_none());
        assert!(listed.setlists.is_none());
        assert_eq!(server.finish().len(), 2);
    }

    #[test]
    fn a_setlists_date_is_read_as_tonepush_keeps_it() {
        assert_eq!(performed_on("4 Oct 2026").as_deref(), Some("2026-10-04"));
        assert_eq!(
            performed_on("28 September 2026").as_deref(),
            Some("2026-09-28")
        );
        assert_eq!(performed_on("Various"), None);
        assert_eq!(performed_on(""), None);
    }
}
