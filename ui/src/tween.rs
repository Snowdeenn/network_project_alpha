use raylib::{color::Color, math::Vector2};

use crate::{arena::NodeId, event::UIEvent, node::UiVec2};

// ==================================
// Fonction d'Easing
// ==================================

pub mod easing {
    pub type EasingFn = fn(f32) -> f32;

    pub fn linear(t: f32) -> f32 {
        t
    }

    pub fn ease_in_quad(t: f32) -> f32 {
        t * t
    }

    pub fn ease_out_quad(t: f32) -> f32 {
        t * (2.0 - t)
    }

    pub  fn ease_in_out_quad(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        }
    }

    pub fn bounce_out(t: f32) -> f32 {
        if t < 1.0 / 2.75 {
            7.5625 * t * t
        } else if t < 2.0 / 2.75 {
            let t = t - 1.5 / 2.75;
            7.5625 * t * t + 0.75
        } else if t < 2.5 / 2.75 {
            let t = t - 2.25 / 2.75;
            7.5625 * t * t + 0.9375
        } else {
            let t = t - 2.625 / 2.75;
            7.5625 * t * t + 0.984375
        }
    }
}

pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let r = from.r as f32 + (to.r as f32 - from.r as f32) * t;
    let g = from.g as f32 + (to.g as f32 - from.g as f32) * t;
    let b = from.b as f32 + (to.b as f32 - from.b as f32) * t;
    let a = from.a as f32 + (to.a as f32 - from.a as f32) * t;

    Color::new(r as u8, g as u8, b as u8, a as u8)
}

pub enum TweenProperty {
    Position { from: Vector2, to: Vector2 },
    Size { from: Vector2, to: Vector2 },
    Opacity { from: f32, to: f32 },
    Color { from: Color, to: Color },
}

pub struct Tween {
    pub property: TweenProperty,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: easing::EasingFn,
    pub target: NodeId,
    pub done: bool,
}

#[derive(Default)]
pub struct TweenEngine {
    tweens: Vec<Tween>,
    event: Vec<UIEvent>,
}

impl TweenEngine {
    pub fn add(&mut self, tween: Tween) {
        self.tweens.push(tween);
    }

    pub(crate) fn update(&mut self, dt: f32) {
        for tween in self.tweens.iter_mut() {
            if tween.done {
                continue;
            }
            tween.elapsed += dt;
            let t = (tween.elapsed / tween.duration).clamp(0.0, 1.0);
            let t_ease = (tween.easing)(t);
            if tween.elapsed >= tween.duration {
                tween.done = true;
            }

            match tween.property {
                TweenProperty::Position { from, to } => {
                    let offset = from + (to - from) * t_ease;
                    self.event.push(UIEvent::SetPosition {
                        target: tween.target,
                        offset: UiVec2::pixels(offset.x, offset.y),
                    });
                }
                TweenProperty::Size { from, to } => {
                    let size = from + (to - from) * t_ease;
                    self.event.push(UIEvent::SetSize {
                        target: tween.target,
                        size: UiVec2::pixels(size.x, size.y),
                    });
                }
                TweenProperty::Opacity { from, to } => {
                    let opacity = from + (to - from) * t_ease;
                    self.event.push(UIEvent::SetOpacity {
                        target: tween.target,
                        opacity,
                    });
                }
                TweenProperty::Color { from, to } => {
                    let color = lerp_color(from, to, t_ease);
                    self.event.push(UIEvent::SetColor {
                        target: tween.target,
                        color,
                    });
                }
            }
        }
        self.tweens.retain(|tween| !tween.done);
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = UIEvent> + '_ {
        self.event.drain(..)
    }
}
