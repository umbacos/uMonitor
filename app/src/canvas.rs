use crate::{model::*, render};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::BTreeSet;

#[derive(Clone, Default)]
pub struct Selection(pub BTreeSet<u64>);
impl Selection {
    pub fn contains(&self, id: u64) -> bool {
        self.0.contains(&id)
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn only(&mut self, id: u64) {
        self.0.clear();
        self.0.insert(id);
    }
    pub fn click(&mut self, id: u64, additive: bool) {
        if additive {
            if !self.0.remove(&id) {
                self.0.insert(id);
            }
        } else if !self.contains(id) {
            self.only(id);
        }
    }
}
pub enum Gesture {
    Move {
        start: Pos2,
        positions: Vec<(u64, f32, f32)>,
    },
    Resize {
        start: Pos2,
        id: u64,
        size: Vec2,
    },
    Marquee {
        start: Pos2,
        base: Selection,
    },
}
pub fn escape(
    ctx: &egui::Context,
    selected: &mut Selection,
    gesture: &mut Option<Gesture>,
) -> bool {
    if !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        return false;
    }
    selected.clear();
    *gesture = None;
    ctx.memory_mut(|memory| {
        if let Some(id) = memory.focused() {
            memory.surrender_focus(id);
        }
    });
    true
}
fn bounds(w: &Widget, origin: Pos2, scale: f32) -> Rect {
    let (x, y, width, height) = render::widget_bounds(w);
    Rect::from_min_size(
        origin + Vec2::new(x, y) * scale,
        Vec2::new(width, height) * scale,
    )
}
fn marquee(
    project: &Project,
    selection: &mut Selection,
    base: &Selection,
    area: Rect,
    origin: Pos2,
    scale: f32,
) {
    *selection = base.clone();
    // Background artwork stays selectable by click, but does not swallow a box selection.
    for w in project.widgets.iter().filter(|w| w.visible && !w.locked) {
        if area.intersects(bounds(w, origin, scale)) {
            selection.0.insert(w.id);
        }
    }
}
/// One canvas response owns hit testing, dragging and selection, so overlapping
/// fields always select the topmost layer and a marquee cannot move a field.
pub fn interact(
    ui: &mut egui::Ui,
    screen: Rect,
    scale: f32,
    project: &mut Project,
    selected: &mut Selection,
    gesture: &mut Option<Gesture>,
) -> Option<&'static str> {
    let response = ui.interact(screen, egui::Id::new("canvas"), Sense::click_and_drag());
    let additive = ui.input(|i| i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command);
    let pointer = ui.input(|i| i.pointer.interact_pos());
    if response.hovered() && ui.input(|i| i.pointer.primary_pressed()) {
        if let Some(pos) = pointer {
            let resize = if selected.0.len() == 1 {
                project.widgets.iter().find(|w| {
                    selected.contains(w.id)
                        && !w.locked
                        && Rect::from_center_size(
                            bounds(w, screen.min, scale).right_bottom(),
                            Vec2::splat(12.),
                        )
                        .contains(pos)
                })
            } else {
                None
            };
            if let Some(w) = resize {
                *gesture = Some(Gesture::Resize {
                    start: pos,
                    id: w.id,
                    size: Vec2::new(w.w, w.h),
                });
            } else {
                // Locked background layers can be clicked, while dragging on them
                // starts a marquee to select the foreground objects above them.
                let hit = project
                    .widgets
                    .iter()
                    .rev()
                    .find(|w| w.visible && bounds(w, screen.min, scale).contains(pos));
                if let Some(w) = hit.filter(|w| !w.locked) {
                    selected.click(w.id, additive);
                    *gesture = if selected.contains(w.id) {
                        Some(Gesture::Move {
                            start: pos,
                            positions: project
                                .widgets
                                .iter()
                                .filter(|w| selected.contains(w.id) && !w.locked)
                                .map(|w| (w.id, w.x, w.y))
                                .collect(),
                        })
                    } else {
                        None
                    };
                } else {
                    let base = if additive {
                        selected.clone()
                    } else {
                        Selection::default()
                    };
                    if !additive {
                        selected.clear();
                    }
                    *gesture = Some(Gesture::Marquee { start: pos, base });
                }
            }
        }
    }
    if let (Some(active), Some(pos)) = (gesture.as_ref(), pointer) {
        match active {
            Gesture::Move { start, positions } => {
                if response.dragged() {
                    let (width, height) = project.dimensions();
                    let delta = (pos - *start) / scale;
                    let snap = |v: f32| {
                        if project.grid {
                            (v / project.grid_size).round() * project.grid_size
                        } else {
                            v.round()
                        }
                    };
                    // Clamp the group as a whole to keep its relative positions.
                    let mut dx = snap(delta.x);
                    let mut dy = snap(delta.y);
                    for (id, x, y) in positions {
                        if let Some(w) = project.widgets.iter().find(|w| w.id == *id) {
                            dx = dx.max(-x).min((width as f32 - w.w).max(*x) - x);
                            dy = dy.max(-y).min((height as f32 - w.h).max(*y) - y);
                        }
                    }
                    for (id, x, y) in positions {
                        if let Some(w) = project.widgets.iter_mut().find(|w| w.id == *id) {
                            w.x = x + dx;
                            w.y = y + dy;
                        }
                    }
                }
            }
            Gesture::Resize { start, id, size } => {
                if response.dragged() {
                    let (width, height) = project.dimensions();
                    if let Some(w) = project.widgets.iter_mut().find(|w| w.id == *id) {
                        let delta = (pos - *start) / scale;
                        w.w = (size.x + delta.x).clamp(8., (width as f32 - w.x).max(8.));
                        w.h = (size.y + delta.y).clamp(8., (height as f32 - w.y).max(8.));
                    }
                }
            }
            Gesture::Marquee { start, base } => {
                if response.dragged() || pos.distance(*start) > 3. {
                    let area = Rect::from_two_pos(*start, pos).intersect(screen);
                    marquee(project, selected, base, area, screen.min, scale);
                    ui.painter().rect_filled(
                        area,
                        0.,
                        Color32::from_rgba_unmultiplied(91, 225, 186, 30),
                    );
                    ui.painter().rect_stroke(
                        area,
                        0.,
                        Stroke::new(1_f32, Color32::from_rgb(91, 225, 186)),
                        egui::StrokeKind::Inside,
                    );
                } else if response.clicked() {
                    if let Some(w) = project
                        .widgets
                        .iter()
                        .rev()
                        .find(|w| w.visible && bounds(w, screen.min, scale).contains(pos))
                    {
                        selected.click(w.id, additive);
                    }
                }
            }
        }
    }
    let mut action = None;
    response.context_menu(|ui| {
        if let Some(pos) = ui.ctx().input(|i| i.pointer.press_origin()) {
            if let Some(w) = project
                .widgets
                .iter()
                .rev()
                .find(|w| w.visible && bounds(w, screen.min, scale).contains(pos))
            {
                if !selected.contains(w.id) {
                    selected.only(w.id);
                }
            }
        }
        if ui.button("Duplicate").clicked() {
            action = Some("duplicate");
            ui.close_menu();
        }
        if ui.button("Delete").clicked() {
            action = Some("delete");
            ui.close_menu();
        }
    });
    for w in project
        .widgets
        .iter()
        .filter(|w| w.visible && selected.contains(w.id))
    {
        let rect = bounds(w, screen.min, scale).expand(3.);
        ui.painter().rect_stroke(
            rect,
            0.,
            Stroke::new(1.5_f32, Color32::from_rgb(91, 225, 186)),
            egui::StrokeKind::Outside,
        );
        if selected.0.len() == 1 && !w.locked {
            ui.painter().rect_filled(
                Rect::from_center_size(rect.right_bottom() - Vec2::splat(3.), Vec2::splat(10.)),
                1.,
                Color32::from_rgb(96, 230, 192),
            );
        }
    }
    if ui.input(|i| i.pointer.primary_released()) {
        *gesture = None;
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Project {
        let mut p = Project::blank();
        p.grid = false;
        for (id, x) in [(1, 40.), (2, 160.), (3, 300.)] {
            let mut w = Widget::new(id, Kind::Text, x, 60.);
            w.w = 80.;
            w.h = 30.;
            p.widgets.push(w);
        }
        p
    }
    fn step(
        ctx: &egui::Context,
        p: &mut Project,
        s: &mut Selection,
        g: &mut Option<Gesture>,
        events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600., 400.))),
            events,
            modifiers,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                escape(ctx, s, g);
                interact(
                    ui,
                    Rect::from_min_size(Pos2::new(20., 20.), Vec2::new(480., 320.)),
                    1.,
                    p,
                    s,
                    g,
                );
            });
        });
    }
    fn button(pos: Pos2, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        }
    }
    fn click(
        ctx: &egui::Context,
        p: &mut Project,
        s: &mut Selection,
        g: &mut Option<Gesture>,
        pos: Pos2,
        mods: egui::Modifiers,
    ) {
        step(ctx, p, s, g, vec![egui::Event::PointerMoved(pos)], mods);
        step(ctx, p, s, g, vec![button(pos, true, mods)], mods);
        step(ctx, p, s, g, vec![button(pos, false, mods)], mods);
    }
    #[test]
    fn pointer_clicks_shift_control_marquee_and_escape() {
        let ctx = egui::Context::default();
        let mut p = fixture();
        let mut s = Selection::default();
        let mut g = None;
        let none = egui::Modifiers::default();
        let shift = egui::Modifiers {
            shift: true,
            ..none
        };
        let ctrl = egui::Modifiers { ctrl: true, ..none };
        step(&ctx, &mut p, &mut s, &mut g, vec![], none);
        click(&ctx, &mut p, &mut s, &mut g, Pos2::new(70., 90.), none);
        assert_eq!(s.0, BTreeSet::from([1]));
        click(&ctx, &mut p, &mut s, &mut g, Pos2::new(190., 90.), shift);
        assert_eq!(s.0, BTreeSet::from([1, 2]));
        click(&ctx, &mut p, &mut s, &mut g, Pos2::new(340., 90.), ctrl);
        assert_eq!(s.0, BTreeSet::from([1, 2, 3]));
        click(&ctx, &mut p, &mut s, &mut g, Pos2::new(190., 90.), ctrl);
        assert_eq!(s.0, BTreeSet::from([1, 3]));
        // Dragging empty canvas boxes the first two objects, with no movement.
        let start = Pos2::new(45., 65.);
        let end = Pos2::new(270., 120.);
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![egui::Event::PointerMoved(start)],
            none,
        );
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![button(start, true, none)],
            none,
        );
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![egui::Event::PointerMoved(end)],
            none,
        );
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![button(end, false, none)],
            none,
        );
        assert_eq!(s.0, BTreeSet::from([1, 2]));
        assert_eq!(p.widgets[0].x, 40.);
        // Group dragging preserves spacing and clears the gesture on release.
        let start = Pos2::new(70., 90.);
        let end = Pos2::new(90., 100.);
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![egui::Event::PointerMoved(start)],
            none,
        );
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![button(start, true, none)],
            none,
        );
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![egui::Event::PointerMoved(end)],
            none,
        );
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![button(end, false, none)],
            none,
        );
        assert_eq!(
            (p.widgets[0].x, p.widgets[1].x, p.widgets[2].x),
            (60., 180., 300.)
        );
        assert!(g.is_none());
        step(
            &ctx,
            &mut p,
            &mut s,
            &mut g,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: none,
            }],
            none,
        );
        assert!(s.0.is_empty());
    }
    #[test]
    fn marquee_respects_visibility_lock_and_rotation_and_adds_to_selection() {
        let mut p = fixture();
        p.widgets[1].locked = true;
        p.widgets[2].visible = false;
        let mut s = Selection::default();
        let base = Selection(BTreeSet::from([99]));
        marquee(
            &p,
            &mut s,
            &base,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(480., 320.)),
            Pos2::ZERO,
            1.,
        );
        assert_eq!(s.0, BTreeSet::from([1, 99]));
    }
}
