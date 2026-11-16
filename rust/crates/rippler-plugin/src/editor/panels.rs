//! The panels of the editor.
//!
//! Port of the layout of `src/PluginEditor.cpp`.
//!
//! The JUCE editor places every widget at a fixed position. This one lays
//! the widgets out in grids instead, but the arrangement is the same: the
//! noise section has two grids of knobs, the mallet section has a column of
//! knobs, the two resonators have a row of four knobs and a row of three,
//! and the coupling section has the mix, the two pitch controls, the bend
//! control, the gain and the meter.

use egui::{Align2, Color32, Grid, Image, Rect, Response, RichText, Sense, Ui, Widget, vec2};
use nice_plug_core::params::enums::Enum;
use rippler_dsp::mallet::MalletType;
use rippler_dsp::models::ModalModel;
use rippler_params::enums::{Coupling, NoiseFilterMode, PartialCount};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::editor::Params;
use crate::editor::knob::{KNOB_HEIGHT, KNOB_WIDTH, Knob, LabelFormat};
use crate::editor::meter::Meter;
use crate::editor::pitch::Pitch;
use crate::editor::settings::Settings;
use crate::editor::tension::Tension;
use crate::editor::theme::Theme;

/// The logo of the plugin, in the light theme.
static LOGO: &[u8] = include_bytes!("../../../../../resources/logo.png");

/// The logo of the plugin, in the dark theme.
static LOGO_DARK: &[u8] = include_bytes!("../../../../../resources/logo_dark.png");

/// The sun, which the theme button shows when the theme is dark.
static SUN: &[u8] = include_bytes!("../../../../../resources/sun.png");

/// The moon, which the theme button shows when the theme is light.
static MOON: &[u8] = include_bytes!("../../../../../resources/moon.png");

/// The button of a resonator that is on.
static ON: &[u8] = include_bytes!("../../../../../resources/on.png");

/// The button of a resonator that is on, in the dark theme.
static ON_DARK: &[u8] = include_bytes!("../../../../../resources/on_dark.png");

/// The button of a resonator that is off.
static OFF: &[u8] = include_bytes!("../../../../../resources/off.png");

/// The button of a resonator that is off, in the dark theme.
static OFF_DARK: &[u8] = include_bytes!("../../../../../resources/off_dark.png");

/// The order the models are shown in.
///
/// The JUCE build shows them in this order, which is not the order the
/// parameter holds them in.
const MODEL_ORDER: [usize; 12] = [0, 1, 2, 10, 3, 4, 5, 11, 6, 9, 7, 8];

/// The order the partial counts are shown in.
const PARTIALS_ORDER: [usize; 7] = [5, 6, 0, 1, 2, 3, 4];

/// The images of the editor.
///
/// The images are the ones of the `resources` folder, and they are decoded
/// when the editor opens.
pub(crate) struct Images {
    /// The logo, in the light theme.
    pub logo: egui::TextureHandle,
    /// The logo, in the dark theme.
    pub logo_dark: egui::TextureHandle,
    /// The sun.
    pub sun: egui::TextureHandle,
    /// The moon.
    pub moon: egui::TextureHandle,
    /// The button of a resonator that is on.
    pub on: egui::TextureHandle,
    /// The button of a resonator that is on, in the dark theme.
    pub on_dark: egui::TextureHandle,
    /// The button of a resonator that is off.
    pub off: egui::TextureHandle,
    /// The button of a resonator that is off, in the dark theme.
    pub off_dark: egui::TextureHandle,
}

impl Images {
    /// Decodes the images of the editor.
    pub(crate) fn load(context: &egui::Context) -> Self {
        Self {
            logo: texture(context, "logo", LOGO),
            logo_dark: texture(context, "logo_dark", LOGO_DARK),
            sun: texture(context, "sun", SUN),
            moon: texture(context, "moon", MOON),
            on: texture(context, "on", ON),
            on_dark: texture(context, "on_dark", ON_DARK),
            off: texture(context, "off", OFF),
            off_dark: texture(context, "off_dark", OFF_DARK),
        }
    }
}

/// Decodes a PNG and turns it into a texture.
fn texture(context: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let image = match image::load_from_memory_with_format(bytes, image::ImageFormat::Png) {
        Ok(image) => {
            let image = image.to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw())
        }
        Err(_) => egui::ColorImage::new([1, 1], vec![Color32::TRANSPARENT]),
    };
    context.load_texture(name, image, egui::TextureOptions::LINEAR)
}

/// What the panels need to draw themselves.
pub(crate) struct Panels<'a> {
    /// The parameters and the setter.
    pub params: &'a Params<'a>,
    /// The colours.
    pub theme: Theme,
    /// Whether the velocity edit mode is on.
    pub velocity_mode: bool,
    /// The images.
    pub images: &'a Images,
    /// The level of the meter.
    pub meter: f32,
    /// The settings that are not plugin parameters.
    pub settings: &'a Arc<Mutex<Settings>>,
    /// The number of voices the plugin plays.
    pub polyphony: &'a Arc<AtomicUsize>,
    /// Whether the velocity edit mode is on.
    pub velocity: &'a Arc<AtomicBool>,
    /// A user sample that the editor decoded, waiting for the audio thread.
    pub user_sample: &'a Arc<Mutex<Option<crate::params::UserSample>>>,
}

impl<'a> Panels<'a> {
    /// A knob bound to a parameter.
    fn knob(&self, id: &'static str, name: &'static str, format: LabelFormat) -> Knob<'a> {
        Knob::new(self.params, id, name, format)
            .with_theme(self.theme)
            .with_velocity_mode(self.velocity_mode)
    }

    /// A tension control bound to a parameter.
    fn tension(&self, id: &'static str, invert: bool) -> Tension<'a> {
        Tension::new(self.params, id, invert).with_theme(self.theme)
    }
}

/// Draws a knob in a grid cell and returns the rectangle it was drawn in.
fn add_knob(ui: &mut Ui, knob: Knob) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(KNOB_WIDTH, KNOB_HEIGHT), Sense::hover());
    knob.ui_at(ui, rect);
    rect
}

/// Draws a tension control in the corner of a knob.
///
/// The JUCE build puts the control in the top right corner of the knob, so
/// that it overlaps the knob next to it a little.
fn add_tension(ui: &mut Ui, tension: Tension, knob: Rect) {
    let corner = Rect::from_min_size(knob.right_top() - vec2(10.0, 0.0), vec2(15.0, 15.0));
    tension.ui_at(ui, corner);
}

/// Draws the label of a section.
fn section_label(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.label(RichText::new(text).color(theme.neutral_light).size(16.0));
}

/// How a combo box is drawn.
struct ComboStyle<'a> {
    /// The choices.
    choices: &'a [&'a str],
    /// The order of the choices.
    ///
    /// An empty order means that the choices are shown in the order the
    /// parameter holds them in.
    order: &'a [usize],
    /// Whether the combo is filled with the active colour.
    filled: bool,
    /// The colours.
    theme: &'a Theme,
    /// The width of the combo.
    width: f32,
}

/// Draws a combo box bound to a choice parameter.
///
/// The `order` says which value of the parameter every entry stands for, so
/// that the entries can be shown in the order the JUCE build shows them in.
fn combo(ui: &mut Ui, params: &Params, id: &str, style: ComboStyle) -> Response {
    let ComboStyle {
        choices,
        order,
        filled,
        theme,
        width,
    } = style;

    // An empty order means that the choices are shown in the order the
    // parameter holds them in.
    let order: Vec<usize> = if order.is_empty() {
        (0..choices.len()).collect()
    } else {
        order.to_vec()
    };

    let current = params.plain(id).round() as usize;
    let mut selected = order
        .iter()
        .position(|index| *index == current)
        .unwrap_or(0);

    let (background, foreground) = if filled {
        (theme.active, theme.on_active())
    } else {
        (theme.background, theme.active)
    };

    let visuals = ui.visuals().clone();
    {
        let visuals = ui.visuals_mut();
        visuals.widgets.inactive.bg_fill = background;
        visuals.widgets.inactive.weak_bg_fill = background;
        visuals.widgets.inactive.fg_stroke.color = foreground;
        visuals.widgets.hovered.bg_fill = background;
        visuals.widgets.hovered.weak_bg_fill = background;
        visuals.widgets.hovered.fg_stroke.color = foreground;
        visuals.widgets.open.bg_fill = background;
        visuals.widgets.open.weak_bg_fill = background;
        visuals.widgets.open.fg_stroke.color = foreground;
    }

    let response = egui::ComboBox::from_id_salt(id)
        .width(width)
        .selected_text(choices[selected])
        .show_ui(ui, |ui| {
            for (position, choice) in choices.iter().enumerate() {
                ui.selectable_value(&mut selected, position, *choice);
            }
        });

    *ui.visuals_mut() = visuals;

    if response.response.changed() {
        params.begin(id);
        params.set_plain(id, order[selected] as f64);
        params.end(id);
    }

    response.response
}

/// A slider bound to a parameter.
///
/// The JUCE build's sliders are bars with the name of the parameter in them.
pub(crate) struct BarSlider<'a> {
    /// The parameters and the setter.
    params: &'a Params<'a>,
    /// The identifier of the parameter the slider edits.
    id: &'static str,
    /// The name in the slider.
    name: &'static str,
    /// The colours.
    theme: Theme,
    /// The width of the slider.
    width: f32,
    /// The height of the slider.
    height: f32,
}

impl<'a> BarSlider<'a> {
    /// Builds a slider.
    pub(crate) fn new(
        params: &'a Params<'a>,
        id: &'static str,
        name: &'static str,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            params,
            id,
            name,
            theme: Theme::LIGHT,
            width,
            height,
        }
    }

    /// Sets the colours of the slider.
    pub(crate) fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for BarSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = vec2(self.width, self.height);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

        let id = ui.make_persistent_id(("slider", self.id));
        let response = ui.interact(rect, id, Sense::click_and_drag());

        let value = self.params.normalized(self.id);
        let modifiers = ui.input(|input| input.modifiers);

        if response.drag_started() {
            self.params.begin(self.id);
        }

        if response.dragged() {
            // The slider follows the mouse along its length.
            let position = response.interact_pointer_pos().map_or(rect.left(), |p| p.x);
            let value = ((position - rect.left()) / rect.width()).clamp(0.0, 1.0);
            self.params.set_normalized(self.id, value);
        }

        if response.drag_stopped() {
            self.params.end(self.id);
        }

        if response.double_clicked() {
            self.params.begin(self.id);
            self.params
                .set_normalized(self.id, self.params.default_normalized(self.id));
            self.params.end(self.id);
        }

        // The wheel steps the slider.
        if response.hovered() {
            let wheel: f32 = ui.input(|input| {
                input
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                        _ => None,
                    })
                    .sum()
            });
            if wheel != 0.0 && !ui.input(|input| input.pointer.any_down()) {
                let step = if modifiers.ctrl { 0.01 } else { 0.05 };
                let change = if wheel > 0.0 { step } else { -step };
                self.params.begin(self.id);
                self.params
                    .set_normalized(self.id, (value + change).clamp(0.0, 1.0));
                self.params.end(self.id);
            }
        }

        // The bar itself.
        let painter = ui.painter();
        painter.rect_filled(
            rect.shrink(0.5),
            3.0,
            Color32::from_rgba_unmultiplied(
                self.theme.active.r(),
                self.theme.active.g(),
                self.theme.active.b(),
                128,
            ),
        );
        let filled = Rect::from_min_max(
            rect.left_top(),
            egui::Pos2::new(rect.left() + rect.width() * value, rect.bottom()),
        );
        painter.rect_filled(filled.shrink(0.5), 3.0, self.theme.active);

        // The name of the slider, or its value while the mouse is down.
        let text = if response.dragged() || response.is_pointer_button_down_on() {
            format!("{} %", (value * 100.0).round() as i32)
        } else {
            String::from(self.name)
        };
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(15.0),
            self.theme.on_active(),
        );

        response
    }
}

/// The noise section.
pub(crate) fn noise_section(ui: &mut Ui, panels: &Panels) {
    ui.horizontal(|ui| {
        section_label(ui, &panels.theme, "NOISE");
        combo(
            ui,
            panels.params,
            "noise_filter_mode",
            ComboStyle {
                choices: NoiseFilterMode::variants(),
                order: &[],
                filled: false,
                theme: &panels.theme,
                width: 60.0,
            },
        );
    });

    Grid::new("noise-knobs")
        .spacing(vec2(0.0, 0.0))
        .show(ui, |ui| {
            ui.add(
                panels
                    .knob("noise_mix", "Mix", LabelFormat::Percent)
                    .with_velocity("vel_noise_mix"),
            );
            ui.add(panels.knob("noise_filter_freq", "Freq", LabelFormat::Hz));
            ui.end_row();
            ui.add(
                panels
                    .knob("noise_res", "Res", LabelFormat::Percent)
                    .with_velocity("vel_noise_res"),
            );
            ui.add(panels.knob("noise_filter_q", "Q", LabelFormat::Float1));
            ui.end_row();
        });

    ui.horizontal(|ui| {
        section_label(ui, &panels.theme, "ADSR");
        ui.add(
            BarSlider::new(panels.params, "noise_osc", "OSC", 80.0, 20.0).with_theme(panels.theme),
        );
    });

    Grid::new("noise-envelope")
        .spacing(vec2(0.0, 0.0))
        .show(ui, |ui| {
            let attack = add_knob(
                ui,
                panels
                    .knob("noise_att", "Attack", LabelFormat::Millis)
                    .with_velocity("vel_noise_att"),
            );
            add_tension(ui, panels.tension("noise_att_ten", false), attack);
            let decay = add_knob(
                ui,
                panels
                    .knob("noise_dec", "Decay", LabelFormat::Millis)
                    .with_velocity("vel_noise_dec"),
            );
            add_tension(ui, panels.tension("noise_dec_ten", true), decay);
            ui.end_row();
            ui.add(
                panels
                    .knob("noise_sus", "Sus", LabelFormat::Percent)
                    .with_velocity("vel_noise_sus"),
            );
            let release = add_knob(
                ui,
                panels
                    .knob("noise_rel", "Release", LabelFormat::Millis)
                    .with_velocity("vel_noise_rel"),
            );
            add_tension(ui, panels.tension("noise_rel_ten", true), release);
            ui.end_row();
        });
}

/// The mallet section.
pub(crate) fn mallet_section(ui: &mut Ui, panels: &Panels) {
    let mallet = panels.params.plain("mallet_type").round() as usize;
    let is_impulse = mallet == MalletType::Impulse as usize;
    let is_sample = mallet >= MalletType::UserFile as usize;

    // The label of the section opens the mallet menu, the way the JUCE
    // editor's does.
    ui.horizontal(|ui| {
        let response = ui.add(
            egui::Button::new(
                RichText::new("MALLET")
                    .color(panels.theme.neutral_light)
                    .size(16.0),
            )
            .fill(panels.theme.background),
        );
        crate::editor::menus::mallet_menu(ui, panels, &response);

        let name = MalletType::variants()
            .get(mallet)
            .copied()
            .unwrap_or("Impulse");
        ui.label(
            RichText::new(name)
                .color(panels.theme.neutral_light)
                .size(11.0),
        );
    });

    Grid::new("mallet-knobs")
        .spacing(vec2(0.0, 0.0))
        .show(ui, |ui| {
            ui.add(
                panels
                    .knob("mallet_mix", "Mix", LabelFormat::Percent)
                    .with_velocity("vel_mallet_mix"),
            );
            ui.end_row();
            ui.add(
                panels
                    .knob("mallet_res", "Res", LabelFormat::Percent)
                    .with_velocity("vel_mallet_res"),
            );
            ui.end_row();
            if is_impulse {
                ui.add(
                    panels
                        .knob("mallet_stiff", "Stiff", LabelFormat::Hz)
                        .with_velocity("vel_mallet_stiff"),
                );
                ui.end_row();
            }
        });

    if is_sample {
        ui.add(
            BarSlider::new(panels.params, "mallet_ktrack", "KTrack", 70.0, 25.0)
                .with_theme(panels.theme),
        );

        Grid::new("mallet-sample")
            .spacing(vec2(0.0, 0.0))
            .show(ui, |ui| {
                ui.add(
                    panels
                        .knob("mallet_pitch", "Pitch", LabelFormat::PitchSemis)
                        .symmetric(),
                );
                ui.end_row();
                ui.add(
                    panels
                        .knob("mallet_filter", "Filter", LabelFormat::FilterLphp)
                        .symmetric(),
                );
                ui.end_row();
            });
    }
}

/// The identifiers of a resonator's parameters.
pub(crate) struct ResonatorIds {
    /// The identifier of the switch that turns the resonator on.
    on: &'static str,
    /// The identifier of the model.
    model: &'static str,
    /// The identifier of the number of partials.
    partials: &'static str,
    /// The identifier of the decay.
    decay: &'static str,
    /// The identifier of the material.
    damp: &'static str,
    /// The identifier of the tone.
    tone: &'static str,
    /// The identifier of the hit position.
    hit: &'static str,
    /// The identifier of the release.
    release: &'static str,
    /// The identifier of the inharmonicity.
    inharm: &'static str,
    /// The identifier of the ratio.
    ratio: &'static str,
    /// The identifier of the low cut.
    cut: &'static str,
    /// The identifier of the radius.
    radius: &'static str,
    /// The identifier of the velocity of the decay.
    velocity_decay: &'static str,
    /// The identifier of the velocity of the material.
    velocity_damp: &'static str,
    /// The identifier of the velocity of the tone.
    velocity_tone: &'static str,
    /// The identifier of the velocity of the hit position.
    velocity_hit: &'static str,
    /// The identifier of the velocity of the inharmonicity.
    velocity_inharm: &'static str,
    /// The name of the section.
    label: &'static str,
}

/// The identifiers of the first resonator.
pub(crate) fn resonator_a() -> ResonatorIds {
    ResonatorIds {
        on: "a_on",
        model: "a_model",
        partials: "a_partials",
        decay: "a_decay",
        damp: "a_damp",
        tone: "a_tone",
        hit: "a_hit",
        release: "a_rel",
        inharm: "a_inharm",
        ratio: "a_ratio",
        cut: "a_cut",
        radius: "a_radius",
        velocity_decay: "vel_a_decay",
        velocity_damp: "vel_a_damp",
        velocity_tone: "vel_a_tone",
        velocity_hit: "vel_a_hit",
        velocity_inharm: "vel_a_inharm",
        label: "A RES",
    }
}

/// The identifiers of the second resonator.
pub(crate) fn resonator_b() -> ResonatorIds {
    ResonatorIds {
        on: "b_on",
        model: "b_model",
        partials: "b_partials",
        decay: "b_decay",
        damp: "b_damp",
        tone: "b_tone",
        hit: "b_hit",
        release: "b_rel",
        inharm: "b_inharm",
        ratio: "b_ratio",
        cut: "b_cut",
        radius: "b_radius",
        velocity_decay: "vel_b_decay",
        velocity_damp: "vel_b_damp",
        velocity_tone: "vel_b_tone",
        velocity_hit: "vel_b_hit",
        velocity_inharm: "vel_b_inharm",
        label: "B RES",
    }
}

/// Whether a model is played with a waveguide rather than with partials.
fn is_tube(model: usize) -> bool {
    model == ModalModel::OpenTube as usize || model == ModalModel::ClosedTube as usize
}

/// Whether a model has a ratio between its two coupling points.
fn has_ratio(model: usize) -> bool {
    model == ModalModel::Beam as usize
        || model == ModalModel::Membrane as usize
        || model == ModalModel::Plate as usize
        || model == ModalModel::Djembe as usize
}

/// A resonator section.
///
/// The two resonators are the same section with different parameters, so
/// the same code draws both of them.
pub(crate) fn resonator_section(ui: &mut Ui, panels: &Panels, ids: &ResonatorIds) {
    let on = panels.params.plain(ids.on) >= 0.5;
    let model = panels.params.plain(ids.model).round() as usize;
    let tube = is_tube(model);

    // The parts of a resonator that is off are dimmed, the way the JUCE
    // build dims them.
    let theme = if on {
        panels.theme
    } else {
        panels.theme.dimmed()
    };

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(ids.label)
                .color(theme.neutral_light)
                .size(16.0),
        );

        let texture = match (on, panels.theme.is_dark()) {
            (true, true) => &panels.images.on_dark,
            (true, false) => &panels.images.on,
            (false, true) => &panels.images.off_dark,
            (false, false) => &panels.images.off,
        };
        let response = ui.add(Image::new(texture).sense(Sense::click()));
        if response.clicked() {
            panels.params.begin(ids.on);
            panels.params.set_plain(ids.on, if on { 0.0 } else { 1.0 });
            panels.params.end(ids.on);
        }
    });

    ui.horizontal(|ui| {
        combo(
            ui,
            panels.params,
            ids.model,
            ComboStyle {
                choices: ModalModel::variants(),
                order: &MODEL_ORDER,
                filled: true,
                theme: &theme,
                width: 100.0,
            },
        );
        if !tube {
            combo(
                ui,
                panels.params,
                ids.partials,
                ComboStyle {
                    choices: PartialCount::variants(),
                    order: &PARTIALS_ORDER,
                    filled: false,
                    theme: &theme,
                    width: 60.0,
                },
            );
        }
    });

    Grid::new(("resonator", ids.label))
        .spacing(vec2(0.0, 0.0))
        .show(ui, |ui| {
            ui.add(
                panels
                    .knob(ids.decay, "Decay", LabelFormat::Seconds2f)
                    .with_velocity(ids.velocity_decay)
                    .with_theme(theme),
            );
            if !tube {
                ui.add(
                    panels
                        .knob(ids.damp, "Material", LabelFormat::Percent)
                        .with_velocity(ids.velocity_damp)
                        .symmetric()
                        .with_theme(theme),
                );
            } else {
                ui.add(
                    panels
                        .knob(ids.radius, "Radius", LabelFormat::Percent)
                        .with_theme(theme),
                );
            }
            ui.add(
                panels
                    .knob(ids.hit, "Hit", LabelFormat::Percent)
                    .with_velocity(ids.velocity_hit)
                    .with_theme(theme),
            );
            ui.add(
                panels
                    .knob(ids.release, "Release", LabelFormat::Percent)
                    .with_theme(theme),
            );
            ui.end_row();

            if !tube {
                ui.add(
                    panels
                        .knob(ids.tone, "Tone", LabelFormat::Percent)
                        .with_velocity(ids.velocity_tone)
                        .symmetric()
                        .with_theme(theme),
                );
                ui.add(
                    panels
                        .knob(ids.inharm, "Inharm", LabelFormat::Float2Times100)
                        .with_velocity(ids.velocity_inharm)
                        .with_theme(theme),
                );
                if has_ratio(model) {
                    ui.add(
                        panels
                            .knob(ids.ratio, "Ratio", LabelFormat::Float1)
                            .with_theme(theme),
                    );
                }
                ui.add(
                    panels
                        .knob(ids.cut, "LowCut", LabelFormat::FilterLphp)
                        .symmetric()
                        .with_theme(theme),
                );
            }
            ui.end_row();
        });
}

/// The coupling section.
pub(crate) fn coupling_section(ui: &mut Ui, panels: &Panels) {
    let serial = panels.params.plain("couple").round() as usize == Coupling::Series as usize;
    let a_on = panels.params.plain("a_on") >= 0.5;
    let b_on = panels.params.plain("b_on") >= 0.5;

    let theme = if a_on && b_on {
        panels.theme
    } else {
        panels.theme.dimmed()
    };

    combo(
        ui,
        panels.params,
        "couple",
        ComboStyle {
            choices: Coupling::variants(),
            order: &[],
            filled: true,
            theme: &theme,
            width: 70.0,
        },
    );

    Grid::new("coupling-knobs")
        .spacing(vec2(0.0, 0.0))
        .show(ui, |ui| {
            if serial {
                ui.add(
                    panels
                        .knob("ab_split", "Split", LabelFormat::Percent)
                        .with_theme(theme),
                );
            } else {
                ui.add(
                    panels
                        .knob("ab_mix", "A:B", LabelFormat::AbMix)
                        .symmetric()
                        .with_theme(theme),
                );
            }
            ui.end_row();
        });

    ui.label(RichText::new("Pitch").color(theme.neutral).size(15.0));
    ui.add(Pitch::new(panels.params, "a_coarse", Some("a_fine")).with_theme(theme));
    ui.add(Pitch::new(panels.params, "b_coarse", Some("b_fine")).with_theme(theme));

    ui.label(RichText::new("PBend").color(theme.neutral).size(15.0));
    ui.add(Pitch::new(panels.params, "bend_range", None).with_theme(theme));

    Grid::new("coupling-gain")
        .spacing(vec2(0.0, 0.0))
        .show(ui, |ui| {
            ui.add(
                panels
                    .knob("gain", "Gain", LabelFormat::Db)
                    .symmetric()
                    .with_theme(theme),
            );
            ui.end_row();
        });

    ui.add(Meter::new(panels.meter).with_theme(panels.theme));
}

/// The numbers of voices the plugin can play.
const POLYPHONIES: [usize; 5] = [1, 4, 8, 12, 16];

/// The top bar of the editor.
///
/// The JUCE editor's top bar has the logo, the theme button, the velocity
/// button, the polyphony menu, the patch menu and the settings button.
pub(crate) fn top_bar(ui: &mut Ui, panels: &Panels) {
    ui.horizontal(|ui| {
        // The logo.
        let logo = if panels.theme.is_dark() {
            &panels.images.logo_dark
        } else {
            &panels.images.logo
        };
        ui.add(Image::new(logo).max_size(vec2(120.0, 16.0)));

        // The theme button, which shows the sun when the theme is dark.
        let theme_image = if panels.theme.is_dark() {
            &panels.images.sun
        } else {
            &panels.images.moon
        };
        let response = ui.add(
            Image::new(theme_image)
                .max_size(vec2(20.0, 20.0))
                .sense(Sense::click()),
        );
        if response.clicked() {
            toggle_theme(ui, panels);
        }

        // The velocity button.
        let velocity = panels.velocity.load(Ordering::Relaxed);
        let response = ui.add(
            egui::Button::new(
                RichText::new("Vel Map")
                    .color(panels.theme.velocity)
                    .size(15.0),
            )
            .selected(velocity),
        );
        if response.clicked() {
            panels.velocity.store(!velocity, Ordering::Relaxed);
            ui.ctx().request_repaint();
        }

        // The polyphony.
        ui.label(
            RichText::new("Voices")
                .color(panels.theme.neutral_light)
                .size(16.0),
        );
        let polyphony = panels.polyphony.load(Ordering::Relaxed);
        let mut selected = POLYPHONIES
            .iter()
            .position(|value| *value == polyphony)
            .unwrap_or(2);
        let response = egui::ComboBox::from_id_salt("polyphony")
            .selected_text(POLYPHONIES[selected].to_string())
            .show_ui(ui, |ui| {
                for (position, value) in POLYPHONIES.iter().enumerate() {
                    ui.selectable_value(&mut selected, position, value.to_string());
                }
            });
        if response.response.changed() {
            set_polyphony(ui, panels, POLYPHONIES[selected]);
        }

        // The patch menu and the settings button.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            let gear = gear_button(ui, panels);
            crate::editor::menus::settings_menu(ui, panels, &gear);
            crate::editor::menus::preset_menu(ui, panels);
        });
    });
}

/// Turns the theme over.
fn toggle_theme(ui: &mut Ui, panels: &Panels) {
    let settings = {
        let mut settings = panels
            .settings
            .lock()
            .expect("the settings are not poisoned");
        settings.dark_theme = !settings.dark_theme;
        *settings
    };
    crate::editor::settings::save(&settings);
    ui.ctx().request_repaint();
}

/// Sets the number of voices the plugin plays.
fn set_polyphony(ui: &mut Ui, panels: &Panels, polyphony: usize) {
    panels.polyphony.store(polyphony, Ordering::Relaxed);
    let settings = {
        let mut settings = panels
            .settings
            .lock()
            .expect("the settings are not poisoned");
        settings.polyphony = polyphony;
        *settings
    };
    crate::editor::settings::save(&settings);
    ui.ctx().request_repaint();
}

/// Draws the gear of the settings button.
///
/// The JUCE build draws the gear itself, with six teeth.
fn gear_button(ui: &mut Ui, panels: &Panels) -> Response {
    let size = vec2(25.0, 25.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let id = ui.make_persistent_id("gear");
    let response = ui.interact(rect, id, Sense::click());

    let painter = ui.painter();
    let centre = rect.center();
    let outer = 10.0;
    let inner = outer / 3.0;
    let core = inner / 1.5;
    let step = std::f32::consts::TAU / 6.0;

    painter.circle_filled(centre, outer, panels.theme.active);
    for index in 0..6 {
        let angle = std::f32::consts::TAU + index as f32 * step;
        let tooth = centre + vec2(angle.cos() * outer, angle.sin() * outer);
        painter.circle_filled(tooth, core, panels.theme.background);
    }
    painter.circle_filled(centre, inner, panels.theme.background);

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Parameters;
    use nice_plug_core::context::PluginApi;
    use nice_plug_core::context::gui::GuiContextInner;
    use nice_plug_core::params::internals::ParamPtr;
    use nice_plug_core::plugin::PluginState;
    use rippler_params::params::RipplerParams;
    use std::collections::BTreeMap;

    /// A context that does nothing.
    ///
    /// The editor can be drawn without a host, which is what the tests do.
    struct Mock;

    impl GuiContextInner for Mock {
        fn plugin_api(&self) -> PluginApi {
            PluginApi::Vst3
        }

        unsafe fn raw_begin_set_parameter(&self, _param: ParamPtr) {}

        unsafe fn raw_set_parameter_normalized(&self, _param: ParamPtr, _value: f32) {}

        unsafe fn raw_end_set_parameter(&self, _param: ParamPtr) {}

        fn get_state(&self) -> PluginState {
            PluginState {
                version: String::new(),
                params: BTreeMap::new(),
                fields: BTreeMap::new(),
            }
        }

        fn set_state(&self, _state: PluginState) {}

        fn request_restart(&self) {}
    }

    /// Draws a part of the editor and counts the knobs it drew.
    ///
    /// Every knob draws a circle of the knob's radius, so the circles of
    /// that radius are the knobs.
    fn knobs(mut draw: impl FnMut(&mut Ui, &Panels)) -> usize {
        let context = egui::Context::default();
        let gui = nice_plug::context::gui::GuiContext::new(Arc::new(Mock));
        let setter = gui.param_setter();
        let parameters = RipplerParams::default();
        let map = Parameters::new(&parameters);
        let images = Images::load(&context);
        let settings = Arc::new(Mutex::new(Settings::default()));
        let polyphony = Arc::new(AtomicUsize::new(8));
        let velocity = Arc::new(AtomicBool::new(false));
        let user_sample = Arc::new(Mutex::new(None));

        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(650.0, 485.0),
                )),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let params = Params::new(&parameters, &map, &setter);
                    let panels = Panels {
                        params: &params,
                        theme: Theme::LIGHT,
                        velocity_mode: false,
                        images: &images,
                        meter: 0.0,
                        settings: &settings,
                        polyphony: &polyphony,
                        velocity: &velocity,
                        user_sample: &user_sample,
                    };
                    draw(ui, &panels);
                });
            },
        );

        // The textures are never painted, so the deltas are dropped on
        // purpose.
        let mut output = output;
        output.textures_delta.clear();

        output
            .shapes
            .iter()
            .filter(|shape| {
                matches!(shape.shape, egui::Shape::Circle(circle) if (circle.radius - crate::editor::knob::RADIUS).abs() < 0.01)
            })
            .count()
    }

    #[test]
    fn the_noise_section_has_eight_knobs() {
        assert_eq!(
            knobs(|ui, panels| {
                noise_section(ui, panels);
            }),
            8
        );
    }

    #[test]
    fn the_mallet_section_has_three_knobs() {
        assert_eq!(
            knobs(|ui, panels| {
                mallet_section(ui, panels);
            }),
            3
        );
    }

    #[test]
    fn a_resonator_section_has_seven_knobs() {
        assert_eq!(
            knobs(|ui, panels| {
                resonator_section(ui, panels, &resonator_a());
            }),
            7
        );
    }

    #[test]
    fn the_coupling_section_has_two_knobs() {
        assert_eq!(
            knobs(|ui, panels| {
                coupling_section(ui, panels);
            }),
            2
        );
    }

    /// Draws the whole editor and returns the rectangle its content is in.
    fn content_bounds() -> egui::Rect {
        let context = egui::Context::default();
        let gui = nice_plug::context::gui::GuiContext::new(Arc::new(Mock));
        let setter = gui.param_setter();
        let parameters = RipplerParams::default();
        let map = Parameters::new(&parameters);
        let images = Images::load(&context);
        let settings = Arc::new(Mutex::new(Settings::default()));
        let polyphony = Arc::new(AtomicUsize::new(8));
        let velocity = Arc::new(AtomicBool::new(false));
        let user_sample = Arc::new(Mutex::new(None));

        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(650.0, 485.0),
                )),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let params = Params::new(&parameters, &map, &setter);
                    let panels = Panels {
                        params: &params,
                        theme: Theme::LIGHT,
                        velocity_mode: false,
                        images: &images,
                        meter: 0.0,
                        settings: &settings,
                        polyphony: &polyphony,
                        velocity: &velocity,
                        user_sample: &user_sample,
                    };
                    top_bar(ui, &panels);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.vertical(|ui| noise_section(ui, &panels));
                        ui.vertical(|ui| mallet_section(ui, &panels));
                        ui.vertical(|ui| {
                            resonator_section(ui, &panels, &resonator_a());
                            resonator_section(ui, &panels, &resonator_b());
                        });
                        ui.vertical(|ui| coupling_section(ui, &panels));
                    });
                });
            },
        );
        // The textures are never painted, so the deltas are dropped on
        // purpose.
        output.textures_delta.clear();

        fn bounds_of(points: &[egui::Pos2]) -> egui::Rect {
            let mut min = points[0];
            let mut max = points[0];
            for point in points {
                min = min.min(*point);
                max = max.max(*point);
            }
            egui::Rect::from_min_max(min, max)
        }

        let mut bounds: Option<egui::Rect> = None;
        for shape in &output.shapes {
            let rect = match &shape.shape {
                egui::Shape::Circle(circle) => egui::Rect::from_center_size(
                    circle.center,
                    vec2(circle.radius * 2.0, circle.radius * 2.0),
                ),
                egui::Shape::Rect(rect) => rect.rect,
                egui::Shape::LineSegment { points, .. } => bounds_of(points),
                egui::Shape::Path(path) => bounds_of(&path.points),
                egui::Shape::Text(text) => text.galley.rect,
                _ => continue,
            };
            bounds = Some(match bounds {
                Some(bounds) => bounds.union(rect),
                None => rect,
            });
        }

        bounds.unwrap_or(egui::Rect::ZERO)
    }

    #[test]
    fn the_editor_fits_in_its_window() {
        // The JUCE editor is 650 by 485, and the editor of this build is
        // the same size.
        let bounds = content_bounds();
        assert!(bounds.max.x <= 650.0, "the editor is {} wide", bounds.max.x);
        assert!(bounds.max.y <= 485.0, "the editor is {} high", bounds.max.y);
    }

    #[test]
    fn the_whole_editor_lays_out_without_panicking() {
        knobs(|ui, panels| {
            top_bar(ui, panels);
            noise_section(ui, panels);
            mallet_section(ui, panels);
            resonator_section(ui, panels, &resonator_a());
            resonator_section(ui, panels, &resonator_b());
            coupling_section(ui, panels);
        });
    }
}
