//! Barre de menus et raccourcis clavier (reprise de ref/puce16-neogeo/ng-gui/menu.rs, sans BIOS ni modèle).
//! Fichier : ouvrir, réinitialiser, pause, avance image, états, capture, quitter ; Affichage : échelle,
//! rapport, rotation, plein écran, lissage, overlay ; Filtres ; Machine : DIP, commandes, son ; Débogage.

use eframe::egui::{self, Key, Modifiers};
use puce_jamma::video::Rotation;

use crate::overlay::{Corner, OverlayPrefs};
use crate::prefs::{Aspect, DisplayPrefs, ScaleMode};

/// Action demandée par l'utilisateur (menu ou raccourci).
#[derive(Clone, Debug, PartialEq)]
pub enum MenuAction {
    OpenGames,
    Reset,
    Pause(bool),
    TogglePause,
    FrameAdvance,
    SaveState,
    LoadState,
    NextSlot,
    Screenshot,
    Quit,
    Scale(ScaleMode),
    Aspect(Aspect),
    /// `None` = rotation annoncée par la carte.
    Rotation(Option<Rotation>),
    Fullscreen(bool),
    ToggleFullscreen,
    Smooth(bool),
    /// Bezels en plein écran.
    Bezels(bool),
    ToggleBezels,
    /// Reflets de la vitre du bezel sur l'image.
    BezelGlass(bool),
    Overlay(bool),
    OverlayFull(bool),
    OverlayCorner(Corner),
    Filters,
    /// Mires de réglage : afficher / quitter, mire et variante suivantes, panneau.
    ToggleMires,
    MireStep(i32),
    MireVariant(i32),
    MirePanel,
    Dips,
    Controls,
    Volume(u8),
    Mute(bool),
    Debugger,
    StatusBar(bool),
}

/// Raccourcis : F3 réinitialiser, Ctrl+P pause (P seul est le Start du joueur 2), F5 sauver l'état,
/// F6 avance image, F7 charger l'état, F8 emplacement suivant, F11 plein écran, F12 capture.
pub fn shortcut(key: Key, m: Modifiers) -> Option<MenuAction> {
    if m.command && key == Key::P {
        return Some(MenuAction::TogglePause);
    }
    if m.command {
        match key {
            Key::B => return Some(MenuAction::ToggleBezels),
            Key::M => return Some(MenuAction::ToggleMires),
            Key::H => return Some(MenuAction::MirePanel),
            Key::PageUp => return Some(MenuAction::MireVariant(-1)),
            Key::PageDown => return Some(MenuAction::MireVariant(1)),
            _ => {}
        }
    }
    if m.any() {
        return None;
    }
    Some(match key {
        Key::F3 => MenuAction::Reset,
        Key::F5 => MenuAction::SaveState,
        Key::F6 => MenuAction::FrameAdvance,
        Key::F7 => MenuAction::LoadState,
        Key::F8 => MenuAction::NextSlot,
        Key::F11 => MenuAction::ToggleFullscreen,
        Key::F12 => MenuAction::Screenshot,
        Key::PageUp => MenuAction::MireStep(-1),
        Key::PageDown => MenuAction::MireStep(1),
        _ => return None,
    })
}

/// Ce que la barre de menus doit refléter.
pub struct MenuState<'a> {
    pub paused: bool,
    pub loaded: bool,
    pub slot: u8,
    pub display: &'a DisplayPrefs,
    /// Échelle effective, ex. « x3 (960x672) ».
    pub scale_info: &'a str,
    pub overlay: OverlayPrefs,
    pub volume: u8,
    pub mute: bool,
    pub has_debugger: bool,
    pub debug_open: bool,
    pub status_bar: bool,
}

fn item(ui: &mut egui::Ui, enabled: bool, text: &str, a: MenuAction, out: &mut Option<MenuAction>) {
    if ui.add_enabled(enabled, egui::Button::new(text)).clicked() {
        *out = Some(a);
        ui.close();
    }
}

fn file_menu(ui: &mut egui::Ui, st: &MenuState, out: &mut Option<MenuAction>) {
    item(ui, true, "Ouvrir un jeu…", MenuAction::OpenGames, out);
    item(ui, st.loaded, "Réinitialiser (F3)", MenuAction::Reset, out);
    let mut paused = st.paused;
    if ui
        .add_enabled(
            st.loaded,
            egui::Checkbox::new(&mut paused, "Pause (Ctrl+P)"),
        )
        .changed()
    {
        *out = Some(MenuAction::Pause(paused));
        ui.close();
    }
    item(
        ui,
        st.loaded && st.paused,
        "Avance image (F6)",
        MenuAction::FrameAdvance,
        out,
    );
    ui.separator();
    item(
        ui,
        st.loaded,
        &format!("Sauver l'état {} (F5)", st.slot),
        MenuAction::SaveState,
        out,
    );
    item(
        ui,
        st.loaded,
        &format!("Charger l'état {} (F7)", st.slot),
        MenuAction::LoadState,
        out,
    );
    item(
        ui,
        true,
        "Emplacement suivant (F8)",
        MenuAction::NextSlot,
        out,
    );
    item(
        ui,
        st.loaded,
        "Capture d'écran (F12)",
        MenuAction::Screenshot,
        out,
    );
    ui.separator();
    item(ui, true, "Quitter", MenuAction::Quit, out);
}

fn display_menu(ui: &mut egui::Ui, st: &MenuState, out: &mut Option<MenuAction>) {
    let d = st.display;
    ui.label(format!("Échelle : {}", st.scale_info));
    ui.horizontal(|ui| {
        if ui
            .selectable_label(d.scale == ScaleMode::Auto, "Auto")
            .clicked()
        {
            *out = Some(MenuAction::Scale(ScaleMode::Auto));
        }
        for n in 1..=6 {
            if ui
                .selectable_label(d.scale == ScaleMode::Fixed(n), format!("x{n}"))
                .clicked()
            {
                *out = Some(MenuAction::Scale(ScaleMode::Fixed(n)));
            }
        }
    });
    ui.horizontal(|ui| {
        for (a, label) in [
            (Aspect::Square, "Pixels carrés"),
            (Aspect::Tv, "Rapport du moniteur"),
        ] {
            if ui.selectable_label(d.aspect == a, label).clicked() {
                *out = Some(MenuAction::Aspect(a));
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Rotation");
        let rots = [
            (None, "Auto"),
            (Some(Rotation::Rot0), "0°"),
            (Some(Rotation::Rot90), "90°"),
            (Some(Rotation::Rot180), "180°"),
            (Some(Rotation::Rot270), "270°"),
        ];
        for (r, label) in rots {
            if ui
                .selectable_label(d.rotation_override == r, label)
                .clicked()
            {
                *out = Some(MenuAction::Rotation(r));
            }
        }
    });
    ui.separator();
    let mut fs = d.fullscreen;
    if ui.checkbox(&mut fs, "Plein écran (F11)").changed() {
        *out = Some(MenuAction::Fullscreen(fs));
    }
    let mut smooth = d.smooth;
    if ui.checkbox(&mut smooth, "Lissage bilinéaire").changed() {
        *out = Some(MenuAction::Smooth(smooth));
    }
    let mut bz = d.bezels;
    if ui
        .checkbox(&mut bz, "Bezels en plein écran (Ctrl+B)")
        .changed()
    {
        *out = Some(MenuAction::Bezels(bz));
    }
    let mut glass = d.bezel_glass;
    if ui
        .add_enabled(
            d.bezels,
            egui::Checkbox::new(&mut glass, "Reflets de la vitre"),
        )
        .changed()
    {
        *out = Some(MenuAction::BezelGlass(glass));
    }
    ui.separator();
    let o = st.overlay;
    let mut on = o.enabled;
    if ui.checkbox(&mut on, "Overlay de performances").changed() {
        *out = Some(MenuAction::Overlay(on));
    }
    let mut full = o.full;
    if ui
        .add_enabled(
            o.enabled,
            egui::Checkbox::new(&mut full, "Overlay détaillé"),
        )
        .changed()
    {
        *out = Some(MenuAction::OverlayFull(full));
    }
    ui.horizontal(|ui| {
        for c in [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomLeft,
            Corner::BottomRight,
        ] {
            if ui.selectable_label(o.corner == c, c.label()).clicked() {
                *out = Some(MenuAction::OverlayCorner(c));
            }
        }
    });
    let mut bar = st.status_bar;
    if ui.checkbox(&mut bar, "Barre d'état").changed() {
        *out = Some(MenuAction::StatusBar(bar));
    }
}

fn machine_menu(ui: &mut egui::Ui, st: &MenuState, out: &mut Option<MenuAction>) {
    item(ui, st.loaded, "DIP switches…", MenuAction::Dips, out);
    item(ui, true, "Commandes…", MenuAction::Controls, out);
    ui.separator();
    let mut vol = st.volume;
    if ui
        .add(egui::Slider::new(&mut vol, 0..=100).text("Volume"))
        .changed()
    {
        *out = Some(MenuAction::Volume(vol));
    }
    let mut mute = st.mute;
    if ui.checkbox(&mut mute, "Son coupé").changed() {
        *out = Some(MenuAction::Mute(mute));
    }
}

/// Dessine la barre de menus et renvoie l'action choisie.
pub fn show(ui: &mut egui::Ui, st: &MenuState) -> Option<MenuAction> {
    let mut out = None;
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("Fichier", |ui| file_menu(ui, st, &mut out));
        ui.menu_button("Affichage", |ui| display_menu(ui, st, &mut out));
        if ui.button("Filtres").clicked() {
            out = Some(MenuAction::Filters);
        }
        if ui
            .button("Mires")
            .on_hover_text("Mires de réglage de l'écran et des filtres (Ctrl+M)")
            .clicked()
        {
            out = Some(MenuAction::ToggleMires);
        }
        ui.menu_button("Machine", |ui| machine_menu(ui, st, &mut out));
        let label = if st.debug_open {
            "Débogage ✓"
        } else {
            "Débogage"
        };
        if ui
            .add_enabled(st.has_debugger, egui::Button::new(label))
            .clicked()
        {
            out = Some(MenuAction::Debugger);
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts() {
        let none = Modifiers::NONE;
        assert_eq!(shortcut(Key::F5, none), Some(MenuAction::SaveState));
        assert_eq!(shortcut(Key::F3, none), Some(MenuAction::Reset));
        assert_eq!(shortcut(Key::F12, none), Some(MenuAction::Screenshot));
        assert_eq!(shortcut(Key::F7, none), Some(MenuAction::LoadState));
        assert_eq!(shortcut(Key::F8, none), Some(MenuAction::NextSlot));
        assert_eq!(shortcut(Key::F6, none), Some(MenuAction::FrameAdvance));
        assert_eq!(shortcut(Key::F11, none), Some(MenuAction::ToggleFullscreen));
        assert_eq!(
            shortcut(Key::P, Modifiers::COMMAND),
            Some(MenuAction::TogglePause)
        );
        // P seul est le Start du joueur 2 ; une touche de jeu n'est pas un raccourci
        assert_eq!(shortcut(Key::P, none), None);
        assert_eq!(shortcut(Key::Z, none), None);
        assert_eq!(shortcut(Key::F5, Modifiers::SHIFT), None);
    }
}
