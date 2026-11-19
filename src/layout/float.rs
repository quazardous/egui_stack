//! Floating roots of the layout: [`Float`], where one is ([`Place`]), the
//! screens it may be on ([`Screen`]), and how a place is brought back onto
//! them ([`Place::repair`]).
//!
//! A float shows a dock of the same [`DockSpace`](crate::DockSpace) as the
//! zones, by name ([`Content::Dock`]), either inside the main window
//! ([`FloatHost::InWindow`]) or in a native window of its own
//! ([`FloatHost::Native`]). Its place is in points, local to the screen it
//! is on, with that screen's scale, so a layout saved on one set of screens
//! loads on another: [`Place::repair`] puts each float back where it can be
//! reached, re-projected to the scale of the screen it ends on.

use egui::{Pos2, Rect, Vec2, pos2, vec2};

use super::Content;

/// A floating root of the layout: a dock (or the application's content)
/// over the main window or in a native window of its own.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(default)
)]
#[non_exhaustive]
pub struct Float {
    /// Its key: stable, saved, unique among the layout's floats. A dock
    /// float is usually keyed by its dock's name.
    pub key: String,
    /// What it shows: [`Content::Dock`] for a dock window.
    pub content: Content,
    /// Inside the main window, or a native window of its own.
    pub host: FloatHost,
    /// Where it is. A [`FloatsArea`](crate::FloatsArea) keeps it current as the user moves
    /// the window; to move a float shown from code, set its rect's corner
    /// ([`Layout::float_mut`](super::Layout::float_mut)): its window goes there in the next
    /// frame, with no [`FloatEvent::Moved`](crate::FloatEvent::Moved) (a native float where
    /// the system lets an application place its windows; not on Wayland).
    pub place: Place,
    /// Its stacking order: higher is in front.
    pub z: u32,
    /// Folded to its title bar (in the main window) or minimized (native).
    pub minimized: bool,
}

impl Default for Float {
    fn default() -> Self {
        Self {
            key: String::new(),
            content: Content::Empty,
            host: FloatHost::default(),
            place: Place::default(),
            z: 0,
            minimized: false,
        }
    }
}

impl Float {
    /// A float keyed `key` showing the dock `dock`, at `place`, hosted by
    /// `host`.
    pub fn dock(
        key: impl Into<String>,
        dock: impl Into<String>,
        host: FloatHost,
        place: Place,
    ) -> Self {
        Self {
            key: key.into(),
            content: Content::Dock(dock.into()),
            host,
            place,
            ..Self::default()
        }
    }

    /// The dock it shows, if any.
    pub fn dock_name(&self) -> Option<&str> {
        match &self.content {
            Content::Dock(name) => Some(name),
            _ => None,
        }
    }
}

/// What hosts a [`Float`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "kebab-case")
)]
#[non_exhaustive]
pub enum FloatHost {
    /// Drawn over the main window, as an area of its own.
    #[default]
    InWindow,
    /// A native window (an egui viewport) the application hosts.
    Native,
}

/// Where a [`Float`] is: a rect in points, local to a screen, with the
/// screen's scale when it was saved.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(default)
)]
#[non_exhaustive]
pub struct Place {
    /// The screen's name, as [`Screen::name`]; `None` when unknown (a
    /// window migrated from an older layout, or a system that does not
    /// tell): it is then read on the first screen, at that screen's scale.
    pub screen: Option<String>,
    /// The window's outer rect, in points, from the screen's top left
    /// corner. While maximized or full screen, the rect it comes back to.
    pub rect: Rect,
    /// The screen's scale (physical pixels per point) the rect was taken
    /// at.
    pub scale: f32,
    /// Maximized.
    pub maximized: bool,
    /// Full screen.
    pub fullscreen: bool,
}

impl Default for Place {
    fn default() -> Self {
        Self::new(Rect::from_min_size(Self::DEFAULT_POS, Self::DEFAULT_SIZE))
    }
}

/// What [`Place::repair`] does when the screen a place names is gone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScreenGone {
    /// It moves onto the first screen, re-projected to its scale and kept
    /// inside it.
    #[default]
    MoveOnto,
    /// It docks back: its tabs go where they came from.
    DockBack,
}

/// What [`Place::repair`] found.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Repaired {
    /// The place can be reached as it is.
    Unchanged,
    /// The place to use instead: on another screen, at another scale,
    /// smaller, or moved so its title bar can be reached.
    Moved(Place),
    /// Its screen is gone and the policy is [`ScreenGone::DockBack`].
    DockBack,
}

/// A screen (monitor) a [`Float`] may be on, as the system reports it.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub struct Screen {
    /// Its name, stable across runs (the connector or the monitor's model).
    pub name: String,
    /// Its rect on the desktop, in physical pixels.
    pub rect: Rect,
    /// Physical pixels per point.
    pub scale: f32,
}

impl Screen {
    /// A screen named `name`, at `rect` on the desktop (physical pixels),
    /// with `scale` physical pixels per point.
    pub fn new(name: impl Into<String>, rect: Rect, scale: f32) -> Self {
        Self {
            name: name.into(),
            rect,
            scale: sane_scale(scale),
        }
    }

    /// Its own rect in points, from its top left corner: what a
    /// [`Place::rect`] on it lies in.
    pub fn local_rect(&self) -> Rect {
        Rect::from_min_size(Pos2::ZERO, self.rect.size() / sane_scale(self.scale))
    }
}

impl Place {
    /// Where a float goes when nothing says otherwise.
    pub const DEFAULT_POS: Pos2 = pos2(64.0, 64.0);
    /// The size a float takes when nothing says otherwise.
    pub const DEFAULT_SIZE: Vec2 = vec2(320.0, 240.0);
    /// The height of a window's title bar, in points: what
    /// [`repair`](Self::repair) keeps on a screen.
    pub const TITLE_BAR: f32 = 32.0;
    /// How much of the title bar's width [`repair`](Self::repair) keeps on
    /// a screen (all of it on a narrower window), in points.
    pub const GRAB: f32 = 64.0;

    /// A place at `rect`, on no screen in particular, at scale 1.
    pub fn new(rect: Rect) -> Self {
        Self {
            screen: None,
            rect,
            scale: 1.0,
            maximized: false,
            fullscreen: false,
        }
    }

    /// A place at `rect` (points, local to `screen`) on `screen`.
    pub fn on(screen: &Screen, rect: Rect) -> Self {
        Self {
            screen: Some(screen.name.clone()),
            scale: sane_scale(screen.scale),
            ..Self::new(rect)
        }
    }

    /// The place of a window at `rect` on the desktop, in physical pixels:
    /// on the screen holding its centre, else the one it overlaps most;
    /// `None` when it is on none of `screens`.
    pub fn from_physical(rect: Rect, screens: &[Screen]) -> Option<Self> {
        let screen = screens
            .iter()
            .find(|screen| screen.rect.contains(rect.center()))
            .or_else(|| {
                screens
                    .iter()
                    .map(|screen| (screen, area(screen.rect.intersect(rect))))
                    .filter(|(_, area)| *area > 0.0)
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(screen, _)| screen)
            })?;
        let scale = sane_scale(screen.scale);
        let min = ((rect.min - screen.rect.min) / scale).to_pos2();
        Some(Self::on(
            screen,
            Rect::from_min_size(min, rect.size() / scale),
        ))
    }

    /// Its rect on the desktop, in physical pixels, on `screen`: the
    /// offset from the screen's corner as it was saved (at
    /// [`scale`](Self::scale)), the size at the screen's scale. Two
    /// screens of different scales thus never make floats overlap or leave
    /// gaps between them.
    pub fn physical(&self, screen: &Screen) -> Rect {
        let offset = self.rect.min.to_vec2() * self.read_scale(screen);
        Rect::from_min_size(
            screen.rect.min + offset,
            self.rect.size() * sane_scale(screen.scale),
        )
    }

    /// Brings the place back onto `screens`, without side effects:
    ///
    /// - on its own screen, re-projected when that screen's scale changed
    ///   (the offset from its corner kept in physical pixels, the size in
    ///   points);
    /// - shrunk to fit the screen;
    /// - moved, as little as it takes, so that its title bar
    ///   ([`TITLE_BAR`](Self::TITLE_BAR) high, at least
    ///   [`GRAB`](Self::GRAB) of its width) is on some screen;
    /// - when its screen is gone, moved onto the first screen and kept
    ///   inside it ([`ScreenGone::MoveOnto`]), or docked back
    ///   ([`ScreenGone::DockBack`]);
    /// - a place with no screen is read on the first one.
    ///
    /// With no screen at all (a system that does not tell), nothing is
    /// changed: a position is never invented.
    pub fn repair(&self, screens: &[Screen], gone: ScreenGone) -> Repaired {
        let Some(first) = screens.first() else {
            return Repaired::Unchanged;
        };
        let named = self
            .screen
            .as_deref()
            .and_then(|name| screens.iter().find(|screen| screen.name == name));
        let (screen, moved) = match (named, &self.screen) {
            (Some(screen), _) => (screen, false),
            (None, Some(_)) => match gone {
                ScreenGone::DockBack => return Repaired::DockBack,
                ScreenGone::MoveOnto => (first, true),
            },
            (None, None) => (first, false),
        };
        let scale = sane_scale(screen.scale);
        let bounds = screen.local_rect();
        let min = (self.rect.min.to_vec2() * self.read_scale(screen) / scale).to_pos2();
        let size = self.rect.size().min(bounds.size()).max(Vec2::ZERO);
        let mut place = Self {
            screen: Some(screen.name.clone()),
            rect: Rect::from_min_size(min, size),
            scale,
            ..self.clone()
        };
        if moved || size != self.rect.size() {
            place.rect = inside(place.rect, bounds);
        } else if !place.title_bar_reachable(screen, screens) {
            place.rect = title_bar_on(place.rect, bounds);
        }
        if place == *self {
            Repaired::Unchanged
        } else {
            Repaired::Moved(place)
        }
    }

    /// The scale its offset is read at on `screen`: its own, or the
    /// screen's when it names no screen.
    fn read_scale(&self, screen: &Screen) -> f32 {
        if self.screen.is_some() {
            sane_scale(self.scale)
        } else {
            sane_scale(screen.scale)
        }
    }

    /// Whether its title bar, placed on `screen`, lies on one of `screens`.
    fn title_bar_reachable(&self, screen: &Screen, screens: &[Screen]) -> bool {
        let scale = sane_scale(screen.scale);
        let rect = self.physical(screen);
        let bar = Rect::from_min_size(
            rect.min,
            vec2(rect.width(), (Self::TITLE_BAR * scale).min(rect.height())),
        );
        let grab = (Self::GRAB * scale).min(bar.width());
        screens.iter().any(|screen| {
            let seen = screen.rect.intersect(bar);
            seen.is_positive() && seen.width() >= grab - 0.5 && seen.height() >= bar.height() - 0.5
        })
    }
}

/// `rect` moved inside `bounds` (it is no larger).
fn inside(rect: Rect, bounds: Rect) -> Rect {
    let x = rect.min.x.clamp(
        bounds.min.x,
        (bounds.max.x - rect.width()).max(bounds.min.x),
    );
    let y = rect.min.y.clamp(
        bounds.min.y,
        (bounds.max.y - rect.height()).max(bounds.min.y),
    );
    Rect::from_min_size(pos2(x, y), rect.size())
}

/// `rect` moved as little as it takes for its title bar to lie in `bounds`.
fn title_bar_on(rect: Rect, bounds: Rect) -> Rect {
    let grab = Place::GRAB.min(rect.width());
    let bar = Place::TITLE_BAR.min(rect.height());
    let x = rect.min.x.clamp(
        bounds.min.x - (rect.width() - grab),
        (bounds.max.x - grab).max(bounds.min.x),
    );
    let y = rect
        .min
        .y
        .clamp(bounds.min.y, (bounds.max.y - bar).max(bounds.min.y));
    Rect::from_min_size(pos2(x, y), rect.size())
}

fn area(rect: Rect) -> f32 {
    if rect.is_positive() { rect.area() } else { 0.0 }
}

/// A scale that divides: 1 for anything not positive and finite.
fn sane_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}
