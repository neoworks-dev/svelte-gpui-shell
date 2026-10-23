//! Canvases, images and icons.

use gpui::prelude::*;
use gpui::{Context, Div, ObjectFit, SharedString, StyledImage, Svg, Transformation, canvas, img, px, radians, svg};

use super::{Inherited, Painter, attribute, is_far_away};
use crate::assets::register_svg;
use crate::icon_theme::{self, Format};
use crate::images::ImageState;
use crate::style::{Dimension, Style};
use crate::tree::{Node, NodeId};
use crate::workbench::Workbench;

impl Painter<'_> {
    /// The canvas's pixels, sized to its box at the window's scale and drawn as an image.
    pub(super) fn canvas_content(&self, element: Div, id: NodeId, style: &Style) -> Div {
        let Some(surface) = self.tree.canvases.get(&id).cloned() else {
            return element;
        };
        let radii = style.radius.map(|radius| px(radius.unwrap_or(0.0)));
        let layer = canvas(
            |bounds, _, _| bounds,
            move |bounds, _, window, _cx| {
                let scale = window.scale_factor();
                let mut surface = surface.borrow_mut();
                let width = (f32::from(bounds.size.width) * scale).round() as u32;
                let height = (f32::from(bounds.size.height) * scale).round() as u32;
                surface.resize(width, height, scale);
                for old in std::mem::take(&mut surface.stale) {
                    window.drop_image(old).ok();
                }
                if let Some(image) = surface.image() {
                    let corners = gpui::Corners {
                        top_left: radii[0],
                        top_right: radii[1],
                        bottom_right: radii[2],
                        bottom_left: radii[3],
                    };
                    window.paint_image(bounds, bounds, corners, image, 0, false).ok();
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        element.child(layer)
    }

    /// An image draws its file once the source has resolved to one, and only while it is
    /// near the window, decoded no larger than it is shown.
    pub(super) fn image_content(&self, mut element: Div, id: NodeId, node: &Node, style: &Style, zoom: f32) -> Div {
        let source = attribute(node, "src");
        if source.is_empty() {
            return element;
        }
        let images = &self.interaction.images;
        let ImageState::Ready(path) = images.state(source) else {
            return element;
        };
        element = element.child(self.measure(id));

        // Until it is drawn the box takes the image's own proportions, so the page does not jump.
        if let (Some((natural_width, natural_height)), None) = (images.dimensions(&path), style.width) {
            if style.height.is_none() {
                element = element.w(px(natural_width as f32 * zoom)).max_w_full();
            }
            if style.aspect_ratio.is_none() {
                element.style().aspect_ratio = Some(natural_width as f32 / natural_height.max(1) as f32);
            }
        }

        let bounds = self.interaction.bounds.borrow().get(&id).copied();
        let view = self.interaction.viewport.get();
        let Some(placed) = bounds.filter(|placed| !is_far_away(*placed, view)) else {
            return element;
        };
        let wanted = f32::from(placed.size.width) * self.interaction.scale.get();
        let bucket = ((wanted.ceil() as u32).max(1)).div_ceil(64) * 64;
        let ImageState::Ready(file) = images.scaled(&path, bucket) else {
            return element;
        };
        let fit = match attribute(node, "fit") {
            "cover" => ObjectFit::Cover,
            "fill" => ObjectFit::Fill,
            _ => ObjectFit::Contain,
        };
        let mut picture = img(file).size_full().object_fit(fit);
        let [top_left, top_right, bottom_right, bottom_left] = style.radius;
        let corners = &mut picture.style().corner_radii;
        corners.top_left = top_left.map(|radius| px(radius).into());
        corners.top_right = top_right.map(|radius| px(radius).into());
        corners.bottom_right = bottom_right.map(|radius| px(radius).into());
        corners.bottom_left = bottom_left.map(|radius| px(radius).into());
        element.child(picture)
    }

    /// An icon: SVG markup the app passed, a file, or a name from the icon theme, tinted with the
    /// text colour unless it is a full-colour theme icon.
    pub(super) fn icon_content(
        &self,
        mut element: Div,
        node: &Node,
        style: &Style,
        inherited: &Inherited,
        cx: &mut Context<Workbench>,
    ) -> Div {
        let side = match style.width {
            Some(Dimension::Px(width)) => width,
            _ => inherited.font_size,
        };
        if style.width.is_none() {
            element = element.w(px(inherited.font_size));
        }
        if style.height.is_none() {
            element = element.h(px(inherited.font_size));
        }
        if let Some(name) = node.attributes.get("name") {
            let fallback = node.attributes.get("fallback");
            return self.themed_icon(element, [Some(name), fallback], node, style, side, inherited, cx);
        }
        let path = match (node.attributes.get("svg"), node.attributes.get("src")) {
            (Some(markup), _) => register_svg(markup.clone()),
            (None, Some(source)) => source.clone(),
            (None, None) => return element,
        };
        element.child(tinted(svg().path(SharedString::from(path)), style, inherited))
    }

    /// The first of `names` the icon theme has. A symbolic icon is tinted; a coloured one is drawn
    /// as it is, from a raster the host makes at the size it is shown.
    fn themed_icon(
        &self,
        element: Div,
        names: [Option<&String>; 2],
        node: &Node,
        style: &Style,
        side: f32,
        inherited: &Inherited,
        cx: &mut Context<Workbench>,
    ) -> Div {
        let scale = self.interaction.scale.get().max(1.0);
        let theme = node.attributes.get("theme").map(String::as_str);
        let found = names
            .into_iter()
            .flatten()
            .find_map(|name| icon_theme::find(name, side.round().max(1.0) as u32, scale.ceil() as u32, theme));
        let Some(file) = found else {
            return element;
        };
        let pixels = ((side * scale).ceil() as u32).max(1).div_ceil(16) * 16;
        match (file.format, file.symbolic) {
            (Format::Svg, true) => {
                let path = SharedString::from(file.path.to_string_lossy().into_owned());
                element.child(tinted(svg().external_path(path), style, inherited))
            }
            (Format::Svg, false) => {
                let image = self.interaction.images.raster(&cx.svg_renderer(), &file.path, pixels);
                match image {
                    Some(image) => element.child(img(image).size_full().object_fit(ObjectFit::Contain)),
                    None => element,
                }
            }
            (Format::Raster, _) => match self.interaction.images.scaled(&file.path, pixels) {
                ImageState::Ready(path) => element.child(img(path).size_full().object_fit(ObjectFit::Contain)),
                _ => element,
            },
        }
    }
}

/// A vector icon drawn in the text colour, turned when the style says so.
fn tinted(mut drawing: Svg, style: &Style, inherited: &Inherited) -> Svg {
    drawing = drawing.size_full().text_color(inherited.colour);
    if let Some(degrees) = style.rotate {
        drawing = drawing.with_transformation(Transformation::rotate(radians(degrees.to_radians())));
    }
    drawing
}
