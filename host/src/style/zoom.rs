//! `scale`: an element drawn larger or smaller about its centre, together with everything inside it.
//!
//! GPUI has no paint-time transform for boxes and text, so the scale is applied to the style
//! instead: every length inside the element is multiplied, and the element keeps the room it
//! had in its parent when its `width` and `height` are given in pixels.

use super::{Dimension, Style};

/// The smallest factor drawn, so a transition that starts at 0 never asks for a zero-sized font.
const SMALLEST: f32 = 0.01;

fn pixels(value: Option<f32>, factor: f32) -> Option<f32> {
    value.map(|pixels| pixels * factor)
}

fn dimension(value: Option<Dimension>, factor: f32) -> Option<Dimension> {
    match value {
        Some(Dimension::Px(pixels)) => Some(Dimension::Px(pixels * factor)),
        other => other,
    }
}

/// Adds `shift` pixels to a margin that is in pixels or unset; shares of the parent and `auto` stay.
fn shifted(margin: Option<Dimension>, shift: f32) -> Option<Dimension> {
    match margin {
        None => Some(Dimension::Px(shift)),
        Some(Dimension::Px(pixels)) => Some(Dimension::Px(pixels + shift)),
        other => other,
    }
}

impl Style {
    /// The style to draw with when the element sits inside ancestors scaled by `outer` and has
    /// its own `scale`; `None` when nothing is scaled. `inherited_font_size` is the font size the
    /// parent passes down, already scaled by `outer`.
    pub fn zoomed(&self, outer: f32, inherited_font_size: f32) -> Option<Style> {
        let own = self.scale.unwrap_or(1.0).max(SMALLEST);
        if outer == 1.0 && own == 1.0 {
            return None;
        }
        let inner = outer * own;
        let mut zoomed = self.clone();
        zoomed.width = dimension(self.width, inner);
        zoomed.height = dimension(self.height, inner);
        zoomed.min_width = dimension(self.min_width, inner);
        zoomed.min_height = dimension(self.min_height, inner);
        zoomed.max_width = dimension(self.max_width, inner);
        zoomed.max_height = dimension(self.max_height, inner);
        zoomed.basis = dimension(self.basis, inner);
        zoomed.row_gap = pixels(self.row_gap, inner);
        zoomed.column_gap = pixels(self.column_gap, inner);
        zoomed.line_height = pixels(self.line_height, inner);
        zoomed.padding = self.padding.map(|side| pixels(side, inner));
        zoomed.border_width = self.border_width.map(|side| pixels(side, inner));
        zoomed.radius = self.radius.map(|corner| pixels(corner, inner));
        zoomed.margin = self.margin.map(|side| dimension(side, outer));
        zoomed.inset = self.inset.map(|side| dimension(side, outer));
        zoomed.translate = self.translate.map(|(x, y)| (x * outer, y * outer));
        zoomed.font_size = match self.font_size {
            Some(size) => Some(size * inner),
            None if own != 1.0 => Some(inherited_font_size * own),
            None => None,
        };
        if let Some(shadows) = &self.shadow {
            zoomed.shadow = Some(
                shadows
                    .iter()
                    .map(|shadow| super::Shadow {
                        x: shadow.x * inner,
                        y: shadow.y * inner,
                        blur: shadow.blur * inner,
                        spread: shadow.spread * inner,
                        color: shadow.color,
                    })
                    .collect(),
            );
        }
        zoomed.keep_room(self, outer, own);
        Some(zoomed)
    }

    /// Margins that give back the room the scaling took from, or added to, a box whose size is
    /// known, so its neighbours stay where they were and it grows from its centre.
    fn keep_room(&mut self, original: &Style, outer: f32, own: f32) {
        if own == 1.0 {
            return;
        }
        if let Some(Dimension::Px(width)) = original.width {
            let shift = width * outer * (1.0 - own) / 2.0;
            self.margin[1] = shifted(self.margin[1], shift);
            self.margin[3] = shifted(self.margin[3], shift);
        }
        if let Some(Dimension::Px(height)) = original.height {
            let shift = height * outer * (1.0 - own) / 2.0;
            self.margin[0] = shifted(self.margin[0], shift);
            self.margin[2] = shifted(self.margin[2], shift);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(width: f32, height: f32) -> Style {
        Style { width: Some(Dimension::Px(width)), height: Some(Dimension::Px(height)), ..Style::default() }
    }

    #[test]
    fn nothing_is_scaled_by_default() {
        assert_eq!(boxed(10.0, 10.0).zoomed(1.0, 14.0), None);
    }

    #[test]
    fn lengths_inside_are_multiplied() {
        let style = Style { padding: [Some(4.0); 4], radius: [Some(8.0); 4], scale: Some(2.0), ..boxed(10.0, 20.0) };
        let zoomed = style.zoomed(1.0, 14.0).expect("scaled");
        assert_eq!(zoomed.width, Some(Dimension::Px(20.0)));
        assert_eq!(zoomed.height, Some(Dimension::Px(40.0)));
        assert_eq!(zoomed.padding, [Some(8.0); 4]);
        assert_eq!(zoomed.radius, [Some(16.0); 4]);
    }

    #[test]
    fn a_sized_box_keeps_its_room_and_grows_from_the_centre() {
        let style = Style { scale: Some(1.5), ..boxed(40.0, 20.0) };
        let zoomed = style.zoomed(1.0, 14.0).expect("scaled");
        assert_eq!(zoomed.margin[3], Some(Dimension::Px(-10.0)));
        assert_eq!(zoomed.margin[1], Some(Dimension::Px(-10.0)));
        assert_eq!(zoomed.margin[0], Some(Dimension::Px(-5.0)));
    }

    #[test]
    fn a_box_without_a_size_grows_with_its_contents() {
        let style = Style { scale: Some(2.0), ..Style::default() };
        let zoomed = style.zoomed(1.0, 14.0).expect("scaled");
        assert_eq!(zoomed.margin, [None; 4]);
        assert_eq!(zoomed.font_size, Some(28.0));
    }

    #[test]
    fn what_is_inside_follows_the_ancestors_scale() {
        let zoomed = boxed(10.0, 10.0).zoomed(2.0, 28.0).expect("scaled");
        assert_eq!(zoomed.width, Some(Dimension::Px(20.0)));
        assert_eq!(zoomed.font_size, None);
        assert_eq!(zoomed.margin, [None; 4]);
    }

    #[test]
    fn margins_and_offsets_follow_only_the_outer_scale() {
        let style = Style {
            margin: [Some(Dimension::Px(4.0)); 4],
            translate: Some((10.0, 0.0)),
            scale: Some(3.0),
            ..Style::default()
        };
        let zoomed = style.zoomed(2.0, 28.0).expect("scaled");
        assert_eq!(zoomed.margin, [Some(Dimension::Px(8.0)); 4]);
        assert_eq!(zoomed.translate, Some((20.0, 0.0)));
    }

    #[test]
    fn scale_zero_stays_drawable() {
        let style = Style { scale: Some(0.0), ..boxed(10.0, 10.0) };
        let zoomed = style.zoomed(1.0, 14.0).expect("scaled");
        assert!(zoomed.font_size.expect("a size") > 0.0);
    }
}
