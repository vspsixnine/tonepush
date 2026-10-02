//! The library as a pane under the editor (docs/design/library-workflow-
//! 2026-10-02, "The frame"): the pedal's presets on the left, what is playing
//! above, what could play below.
//!
//! The library used to be a page of its own, one switch away from the
//! preset being played, so what you browsed was never what you heard. It is
//! now a pane along the bottom of the main column: a splitter on its top edge
//! resizes it, its grip (or Ctrl L) folds it to its tabs, and it keeps its
//! height and whether it is folded per size of window. It never takes height
//! from the board, which is what says what is playing: it takes it from the
//! block pane, whose knobs step down, fold into one row, then leave only the
//! block's head.

use egui::{CornerRadius, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::config::PaneSize;
use crate::shell;
use crate::theme::{self, Icon, Tier};
use crate::{App, LibraryView};

/// The pane's header: its tabs and its tools.
pub(crate) const HEAD: f32 = 40.0;
/// The least an open pane shows: its header, the table's header and three
/// rows.
pub(crate) const MIN_OPEN: f32 = HEAD + TABLE_HEADER + 3.0 * TABLE_ROW;
/// What the block pane keeps however tall the library is: the block's head,
/// with its name and its lens switch.
pub(crate) const BLOCK_HEAD: f32 = 54.0;
/// The tables in the pane: a row, and the header of captions over them.
pub(crate) const TABLE_ROW: f32 = 34.0;
pub(crate) const TABLE_HEADER: f32 = 30.0;

/// The pane's height, open, at each size of window: the design's defaults.
pub(crate) fn default_height(tier: Tier) -> f32 {
    tier.pick(186.0, 232.0, 512.0)
}

/// The settings file's name for a size of window.
fn tier_key(tier: Tier) -> &'static str {
    match tier {
        Tier::S => "small",
        Tier::M => "medium",
        Tier::L => "large",
    }
}

/// Which of TonePush's tones the Cloud shows: everyone's, or what this
/// library published.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CloudScope {
    #[default]
    Everyone,
    Mine,
}

/// What the pane holds between frames that is not a setting.
#[derive(Default)]
pub(crate) struct PaneState {
    /// A drag of the splitter: the height it started at and where the
    /// pointer was.
    drag: Option<(f32, f32)>,
    /// Opened while the pedal's own pages show, where it starts folded.
    pub open_on_pedal: bool,
    /// On the smallest window the details open over the table.
    pub details: bool,
    /// Put the keyboard in the open tab's search on the next frame.
    pub focus_search: bool,
    /// Everyone's tones, or yours.
    pub cloud_scope: CloudScope,
}

impl App {
    /// The pane as it was left at this size of window.
    pub(crate) fn pane_size(&self, tier: Tier) -> PaneSize {
        self.config
            .library_pane
            .get(tier_key(tier))
            .copied()
            .unwrap_or(PaneSize {
                height: default_height(tier),
                folded: false,
            })
    }

    fn store_pane_size(&mut self, tier: Tier, size: PaneSize) {
        if self.pane_size(tier) != size {
            self.config
                .library_pane
                .insert(tier_key(tier).to_owned(), size);
            self.config.save();
        }
    }

    /// Whether the pane shows only its tabs: folded by hand, or the pedal's
    /// own pages showing (where it starts folded), or a firmware update
    /// holding the pedal, when nothing can be auditioned.
    pub(crate) fn library_folded(&self, tier: Tier) -> bool {
        if self.pro_active() && self.pro.updating() {
            return true;
        }
        if self.page == shell::Page::Pedal {
            return !self.pane.open_on_pedal;
        }
        self.pane_size(tier).folded
    }

    /// Fold the pane to its tabs, or open it (Ctrl L).
    pub(crate) fn toggle_library(&mut self, tier: Tier) {
        if self.page == shell::Page::Pedal {
            self.pane.open_on_pedal = !self.pane.open_on_pedal;
            return;
        }
        let mut size = self.pane_size(tier);
        size.folded = !size.folded;
        self.store_pane_size(tier, size);
    }

    /// Show one of the library's views, opening the pane if it is folded.
    pub(crate) fn open_library(&mut self, view: LibraryView, tier: Tier) {
        self.show_library_view(view);
        if self.page == shell::Page::Pedal {
            self.pane.open_on_pedal = true;
            return;
        }
        let mut size = self.pane_size(tier);
        if size.folded {
            size.folded = false;
            self.store_pane_size(tier, size);
        }
    }

    /// The library pane along the bottom of the main column, below whatever
    /// the page put above it. Called after the page's top panels and before
    /// its central one, so it knows how much room there is and takes none
    /// from the board.
    pub(crate) fn library_pane(&mut self, root: &mut Ui, tier: Tier) {
        let room = root.available_rect_before_wrap().height();
        let folded = self.library_folded(tier);
        // The audition bar, or the strip in its place, rides on top of the
        // pane, folded or not: it is the way out of an audition.
        let bar = if self.bar_shown() {
            crate::audition::BAR
        } else {
            0.0
        };
        // What the rest of the column keeps: the block's head on the Edit
        // page, a useful part of a page or the connect page otherwise.
        let keep = match self.page {
            shell::Page::Edit if self.shows_connect() => 220.0,
            shell::Page::Edit => BLOCK_HEAD,
            shell::Page::Pedal => shell::PAGE_HEAD_WITH_TABS + 96.0,
        };
        let max = (room - keep - bar).max(MIN_OPEN.min(room - HEAD - bar));
        let stored = self.pane_size(tier).height;
        let height = if folded {
            HEAD
        } else {
            stored.clamp(MIN_OPEN.min(max), max.max(HEAD))
        };
        egui::Panel::bottom("library-pane")
            .exact_size(height + bar + 1.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(theme::bg()))
            .show(root, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let full = ui.max_rect();
                ui.painter().hline(
                    full.x_range(),
                    full.top() + 0.5,
                    Stroke::new(1.0, theme::line_strong()),
                );
                if bar > 0.0 {
                    let strip = Rect::from_min_size(
                        Pos2::new(full.left(), full.top() + 1.0),
                        Vec2::new(full.width(), bar),
                    );
                    self.audition_bar(ui, strip, tier);
                }
                let head = Rect::from_min_size(
                    Pos2::new(full.left(), full.top() + 1.0 + bar),
                    Vec2::new(full.width(), HEAD),
                );
                self.pane_head(ui, head, tier, folded);
                if !folded {
                    ui.painter().hline(
                        full.x_range(),
                        head.bottom() - 0.5,
                        Stroke::new(1.0, theme::line()),
                    );
                    let body = Rect::from_min_max(Pos2::new(full.left(), head.bottom()), full.max);
                    let mut body_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(body)
                            .id_salt("library-pane-body")
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    body_ui.set_clip_rect(body);
                    self.pane_body(&mut body_ui, tier);
                }
                self.pane_grip(ui, full, tier, height, max);
            });
    }

    /// The splitter on the pane's top edge, with its grip in the middle:
    /// dragged, it resizes the pane; double-clicked, it folds it.
    fn pane_grip(&mut self, ui: &mut Ui, full: Rect, tier: Tier, height: f32, max: f32) {
        let edge = Rect::from_min_max(
            Pos2::new(full.left(), full.top() - 3.0),
            Pos2::new(full.right(), full.top() + 4.0),
        );
        let response = ui
            .interact(
                edge,
                ui.id().with("library-splitter"),
                Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeVertical)
            .on_hover_text("Drag to resize the library; double-click to fold it (Ctrl L)");
        let grip = Rect::from_center_size(
            Pos2::new(full.center().x, full.top() + 0.5),
            Vec2::new(34.0, 4.0),
        );
        ui.painter()
            .rect_filled(grip.expand(2.0), CornerRadius::same(4), theme::bg());
        ui.painter().rect_filled(
            grip,
            CornerRadius::same(2),
            if response.hovered() || response.dragged() {
                theme::muted()
            } else {
                theme::line_strong()
            },
        );
        if response.double_clicked() {
            self.toggle_library(tier);
            self.pane.drag = None;
            return;
        }
        if response.drag_started() {
            let y = ui
                .input(|input| input.pointer.interact_pos())
                .map_or(full.top(), |pointer| pointer.y);
            self.pane.drag = Some((height, y));
        }
        if let (true, Some((start, from))) = (response.dragged(), self.pane.drag) {
            let Some(pointer) = ui.input(|input| input.pointer.interact_pos()) else {
                return;
            };
            let wanted = start + (from - pointer.y);
            let mut size = self.pane_size(tier);
            if self.page == shell::Page::Pedal {
                self.pane.open_on_pedal = wanted > HEAD + 24.0;
            }
            if wanted < MIN_OPEN / 2.0 {
                // Dragged most of the way down: folded, at the height it had.
                size.folded = true;
            } else {
                size.folded = false;
                size.height = wanted.clamp(MIN_OPEN.min(max), max.max(MIN_OPEN));
            }
            // Kept in memory while dragging, written once it is let go.
            self.config
                .library_pane
                .insert(tier_key(tier).to_owned(), size);
        }
        if response.drag_stopped() {
            self.pane.drag = None;
            self.config.save();
        }
    }

    /// The pane's header: its tabs, the Cloud's Everyone and Mine, and the
    /// open tab's search and filters; folded, the tabs and a way back.
    fn pane_head(&mut self, ui: &mut Ui, rect: Rect, tier: Tier, folded: bool) {
        let inner = rect.shrink2(Vec2::new(10.0, 0.0));
        // The tabs first, so they keep their room: the tools give way, the
        // search narrowing before anything else does.
        let tabs = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .id_salt("library-pane-tabs")
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    self.pane_tabs(ui, tier, folded);
                    if !folded && self.lib_showing == LibraryView::Cloud {
                        ui.add_space(8.0);
                        let (line, _) =
                            ui.allocate_exact_size(Vec2::new(1.0, 20.0), Sense::hover());
                        ui.painter().vline(
                            line.center().x,
                            line.y_range(),
                            Stroke::new(1.0, theme::line()),
                        );
                        ui.add_space(10.0);
                        self.cloud_scope_switch(ui);
                    }
                },
            )
            .response
            .rect;
        let right = Rect::from_min_max(
            Pos2::new((tabs.right() + 12.0).min(inner.right()), inner.top()),
            inner.max,
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(right)
                .id_salt("library-pane-tools")
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                if folded {
                    self.folded_tools(ui, tier);
                } else {
                    self.pane_tools(ui, tier);
                }
            },
        );
    }

    /// Tones, Setlists and Cloud, with their counts. A tab opens the pane
    /// on it when it is folded.
    fn pane_tabs(&mut self, ui: &mut Ui, tier: Tier, folded: bool) {
        let cloud_count = self.cloud_total.unwrap_or(self.cloud_entries.len());
        let tabs = [
            (
                LibraryView::Tones,
                None,
                "Tones",
                Some(self.scoped_tone_count()),
            ),
            (
                LibraryView::Setlists,
                None,
                "Setlists",
                Some(self.scoped_setlist_count()),
            ),
            (
                LibraryView::Cloud,
                Some(Icon::Cloud),
                "Cloud",
                (cloud_count > 0).then_some(cloud_count),
            ),
        ];
        for (view, icon, label, count) in tabs {
            let on = self.lib_showing == view;
            let count = count.map(|count| crate::format_count(count as u64));
            if pane_tab(ui, icon, label, count.as_deref(), on, !folded).clicked() {
                self.open_library(view, tier);
            }
        }
    }

    /// Everyone's tones, or yours: what the Cloud tab lists.
    fn cloud_scope_switch(&mut self, ui: &mut Ui) {
        let mine = self.mine_count().to_string();
        let segments = [
            theme::Segment::new("Everyone").icon(Icon::Users),
            theme::Segment::new("Mine").icon(Icon::User).suffix(mine),
        ];
        let chosen = match self.pane.cloud_scope {
            CloudScope::Everyone => 0,
            CloudScope::Mine => 1,
        };
        if let Some(index) =
            theme::segmented(ui, "cloud-scope", &segments, Some(chosen), false).clicked
        {
            self.pane.cloud_scope = if index == 0 {
                CloudScope::Everyone
            } else {
                CloudScope::Mine
            };
        }
    }

    /// The folded pane's way back: its name and its key.
    fn folded_tools(&mut self, ui: &mut Ui, tier: Tier) {
        if theme::IconButton::new(Icon::PanelBottomOpen)
            .show(ui)
            .on_hover_text("Show the library (Ctrl L)")
            .clicked()
        {
            self.toggle_library(tier);
        }
        theme::keycaps(ui, &["Ctrl L"]);
        if self.pro_active() && self.pro.updating() {
            theme::label(
                ui,
                "Auditions wait for the firmware update",
                theme::regular(12.5),
                theme::muted(),
            );
        } else {
            theme::label(ui, "Library", theme::regular(12.5), theme::muted());
        }
    }

    /// The open tab's tools, laid out from the right edge in: the fold
    /// button, the account, the columns, the pedal scope, the tag filter (or
    /// the Cloud's order), and the search. On the smallest window the
    /// filter, scope, columns and account share one menu, and the details
    /// open over the table from their own button.
    fn pane_tools(&mut self, ui: &mut Ui, tier: Tier) {
        if theme::IconButton::new(Icon::PanelBottomClose)
            .show(ui)
            .on_hover_text("Fold the library to its tabs (Ctrl L)")
            .clicked()
        {
            self.toggle_library(tier);
        }
        let showing = self.lib_showing;
        let mine = showing == LibraryView::Cloud && self.pane.cloud_scope == CloudScope::Mine;
        if tier == Tier::S {
            if showing != LibraryView::Setlists {
                let details = theme::IconButton::new(Icon::Info)
                    .on(self.pane.details)
                    .show(ui)
                    .on_hover_text(if self.pane.details {
                        "Hide the details"
                    } else {
                        "Show the chosen tone's details"
                    });
                if details.clicked() {
                    self.pane.details = !self.pane.details;
                }
            }
            let tools = theme::IconButton::new(Icon::SlidersHorizontal)
                .show(ui)
                .on_hover_text("Filter, pedals, columns and account");
            egui::Popup::menu(&tools)
                .align(egui::RectAlign::BOTTOM_END)
                .gap(4.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    theme::menu_width(ui, 240.0);
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    egui::Frame::new()
                        .inner_margin(egui::Margin::same(6))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                if showing != LibraryView::Setlists && !mine {
                                    self.filter_button(ui);
                                }
                                self.scope_button(ui);
                                if showing == LibraryView::Tones {
                                    self.columns_button(ui);
                                }
                                self.account_button(ui);
                            });
                        });
                });
        } else {
            // The Cloud tab's Everyone and Mine take the room the account's
            // name had, so at the middle size it is an icon.
            self.account_control(ui, showing == LibraryView::Cloud && tier == Tier::M);
            if showing == LibraryView::Tones || (showing == LibraryView::Cloud && tier == Tier::L) {
                self.columns_button(ui);
            }
            self.scope_button(ui);
            if showing != LibraryView::Setlists && !mine {
                self.filter_button(ui);
            }
        }
        if showing == LibraryView::Cloud
            && (self.cloud_searching.is_some() || self.cloud_download.is_some())
        {
            let (spot, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            shell::spin(ui, spot.center(), 5.5);
        }
        let width: f32 = match showing {
            LibraryView::Cloud => tier.pick(140.0, 168.0, 260.0),
            _ => tier.pick(150.0, 196.0, 260.0),
        };
        let tones = self.scoped_tone_count();
        let (query, hint) = match (showing, mine) {
            (LibraryView::Tones, _) => (
                &mut self.tone_search,
                match tones {
                    1 => "Search 1 tone".to_owned(),
                    tones => format!("Search {tones} tones"),
                },
            ),
            (LibraryView::Setlists, _) => (&mut self.setlist_search, "Search setlists".to_owned()),
            (LibraryView::Cloud, true) => (
                &mut self.mine_search,
                "Search what you published".to_owned(),
            ),
            (LibraryView::Cloud, false) => (&mut self.library_search, "Search TonePush".to_owned()),
        };
        // Narrowed when the tabs leave less room than the design gives it,
        // and left out where it would be too narrow to type in.
        let width = width.min(ui.available_width());
        if width < 96.0 {
            return;
        }
        let search = theme::search_field(ui, "library-search", query, &hint, width);
        if std::mem::take(&mut self.pane.focus_search) {
            search.request_focus();
        }
        if showing == LibraryView::Cloud && !mine {
            if search.changed() {
                self.cloud_search_due =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(350));
            }
            if search.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                self.cloud_search_due = Some(std::time::Instant::now());
            }
        }
    }

    /// What the open tab holds: tones or TonePush's as a table beside the
    /// details, or the setlists beside the one chosen.
    fn pane_body(&mut self, ui: &mut Ui, tier: Tier) {
        match self.lib_showing {
            LibraryView::Setlists => self.setlists_view(ui, tier),
            LibraryView::Cloud if self.pane.cloud_scope == CloudScope::Mine => {
                self.mine_view(ui, tier)
            }
            LibraryView::Tones | LibraryView::Cloud => self.library_body(ui, tier),
        }
    }

    /// The pane's keys: Ctrl L folds and opens it, Ctrl 1, 2 and 3 open its
    /// tabs, and Ctrl F searches the open one. Skipped while a field has the
    /// keyboard.
    pub(crate) fn pane_shortcuts(&mut self, ctx: &egui::Context, tier: Tier) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        let pressed = |key: Key| {
            ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, key)))
        };
        if pressed(Key::L) {
            self.toggle_library(tier);
        }
        for (key, view) in [
            (Key::Num1, LibraryView::Tones),
            (Key::Num2, LibraryView::Setlists),
            (Key::Num3, LibraryView::Cloud),
        ] {
            if pressed(key) {
                self.open_library(view, tier);
            }
        }
        if pressed(Key::F) {
            let view = self.lib_showing;
            self.open_library(view, tier);
            self.pane.focus_search = true;
        }
    }
}

/// One of the pane's tabs: its label and count, the chosen one underlined.
/// Folded, none is underlined: the pane shows none of them.
fn pane_tab(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    count: Option<&str>,
    on: bool,
    underline: bool,
) -> egui::Response {
    let label_width = shell::galley(ui, label, theme::medium(13.0), theme::text())
        .size()
        .x;
    let count_width = count.map(|count| {
        shell::galley(ui, count, theme::medium(12.0), theme::text())
            .size()
            .x
    });
    let mut width = 20.0 + label_width;
    if icon.is_some() {
        width += 15.0 + 7.0;
    }
    if let Some(count_width) = count_width {
        width += 7.0 + count_width;
    }
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let ink = if on {
        theme::text()
    } else if response.hovered() {
        theme::text_soft()
    } else {
        theme::muted()
    };
    let galley = shell::galley(ui, label, theme::medium(13.0), ink);
    let count = count.map(|count| {
        shell::galley(
            ui,
            count,
            theme::medium(12.0),
            if on { theme::muted() } else { theme::faint() },
        )
    });
    let mut x = rect.left() + 10.0;
    let y = rect.center().y;
    if let Some(icon) = icon {
        theme::paint_icon(ui, icon, Pos2::new(x + 7.5, y), 15.0, ink);
        x += 15.0 + 7.0;
    }
    let label_width = galley.size().x;
    shell::paint_line(ui, galley, x, y);
    x += label_width;
    if let Some(count) = count {
        shell::paint_line(ui, count, x + 7.0, y);
    }
    if on && underline {
        let bar = Rect::from_min_max(
            Pos2::new(rect.left() + 10.0, rect.bottom() + 4.0),
            Pos2::new(rect.right() - 10.0, rect.bottom() + 6.0),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::same(2), theme::text());
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Connection;

    fn app() -> App {
        let (to_device, _sent) = std::sync::mpsc::channel();
        let (_silent, from_device) = std::sync::mpsc::channel();
        App::new(&egui::Context::default(), to_device, from_device)
    }

    #[test]
    fn the_pane_opens_at_the_designs_height_for_each_size_of_window() {
        let mut app = app();
        app.config.library_pane.clear();
        assert_eq!(app.pane_size(Tier::S).height, 186.0);
        assert_eq!(app.pane_size(Tier::M).height, 232.0);
        assert_eq!(app.pane_size(Tier::L).height, 512.0);
        assert!(!app.library_folded(Tier::M));
    }

    /// Ctrl L folds the pane and opens it again; a size of window keeps its
    /// own pane, and opening a tab opens a folded pane.
    #[test]
    fn the_pane_folds_and_opens_per_size_of_window() {
        let mut app = app();
        app.config.library_pane.clear();
        app.toggle_library(Tier::M);
        assert!(app.library_folded(Tier::M));
        assert!(
            !app.library_folded(Tier::L),
            "the large window kept its own"
        );
        app.open_library(LibraryView::Setlists, Tier::M);
        assert!(!app.library_folded(Tier::M));
        assert_eq!(app.lib_showing, LibraryView::Setlists);
    }

    /// The pedal's own pages start with the pane folded, and opening it
    /// there leaves the editor's pane as it was.
    #[test]
    fn the_pedal_pages_start_with_the_library_folded() {
        let mut app = app();
        app.config.library_pane.clear();
        app.connection = Connection::Online;
        app.go_to(shell::Page::Pedal);
        assert!(app.library_folded(Tier::M));
        app.toggle_library(Tier::M);
        assert!(!app.library_folded(Tier::M));
        app.go_to(shell::Page::Edit);
        assert!(!app.library_folded(Tier::M));
        assert!(!app.pane_size(Tier::M).folded);
    }
}
