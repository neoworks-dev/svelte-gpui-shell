//! Reading a style object's properties, and the values inside them.

use gpui::{CursorStyle, Rgba, SharedString};
use serde_json::Value;

use super::{
    Align, Dimension, Easing, Gradient, Justify, MONO, SANS, Shadow, Sides, Stop, Style, TextAlign, Transition,
};

impl Style {
    /// Sets one property from the style object. Returns false when it is not understood,
    /// which the runtime's types should already have caught.
    pub(super) fn set(&mut self, name: &str, value: &Value) -> bool {
        match name {
            "display" => self.hidden = Some(value.as_str() == Some("none")),
            "direction" => self.row = Some(value.as_str() == Some("row")),
            "wrap" => self.wrap = value.as_bool(),
            "grow" => self.grow = number(value),
            "shrink" => self.shrink = number(value),
            "basis" => self.basis = dimension(value),
            "flex" => {
                // `flex: 1` grows from nothing, as React Native has it.
                let Some(amount) = number(value) else {
                    return false;
                };
                self.grow = Some(amount);
                self.shrink = Some(1.0);
                self.basis = Some(Dimension::Px(0.0));
            }
            "align" => self.align_items = align(value),
            "alignSelf" => self.align_self = align(value),
            "justify" => {
                self.justify = match value.as_str() {
                    Some("start") => Some(Justify::Start),
                    Some("end") => Some(Justify::End),
                    Some("center") => Some(Justify::Center),
                    Some("between") => Some(Justify::Between),
                    Some("around") => Some(Justify::Around),
                    _ => return false,
                }
            }
            "gap" => {
                self.row_gap = number(value);
                self.column_gap = number(value);
            }
            "rowGap" => self.row_gap = number(value),
            "columnGap" => self.column_gap = number(value),
            "padding" => self.padding = sides(value).map(|side| side.and_then(pixels_of)),
            "paddingX" => [self.padding[1], self.padding[3]] = [number(value); 2],
            "paddingY" => [self.padding[0], self.padding[2]] = [number(value); 2],
            "paddingTop" => self.padding[0] = number(value),
            "paddingRight" => self.padding[1] = number(value),
            "paddingBottom" => self.padding[2] = number(value),
            "paddingLeft" => self.padding[3] = number(value),
            "margin" => self.margin = sides(value),
            "marginX" => [self.margin[1], self.margin[3]] = [dimension(value); 2],
            "marginY" => [self.margin[0], self.margin[2]] = [dimension(value); 2],
            "marginTop" => self.margin[0] = dimension(value),
            "marginRight" => self.margin[1] = dimension(value),
            "marginBottom" => self.margin[2] = dimension(value),
            "marginLeft" => self.margin[3] = dimension(value),
            "width" => self.width = dimension(value),
            "height" => self.height = dimension(value),
            "size" => {
                self.width = dimension(value);
                self.height = dimension(value);
            }
            "minWidth" => self.min_width = dimension(value),
            "minHeight" => self.min_height = dimension(value),
            "maxWidth" => self.max_width = dimension(value),
            "maxHeight" => self.max_height = dimension(value),
            "aspectRatio" => self.aspect_ratio = number(value),
            "position" => self.absolute = Some(value.as_str() == Some("absolute")),
            "inset" => self.inset = sides(value),
            "top" => self.inset[0] = dimension(value),
            "right" => self.inset[1] = dimension(value),
            "bottom" => self.inset[2] = dimension(value),
            "left" => self.inset[3] = dimension(value),
            "overflow" => self.clip = Some(value.as_str() == Some("hidden")),
            "translate" => {
                let parts = sides_list(value);
                let x = parts.first().copied().flatten().unwrap_or(0.0);
                let y = parts.get(1).copied().flatten().unwrap_or(0.0);
                self.translate = Some((x, y));
            }
            "scale" => self.scale = number(value),
            "rotate" => self.rotate = number(value),
            "transition" => self.transition = transition(value),
            "background" => match value {
                Value::Object(_) => match gradient(value) {
                    Some(found) => self.gradient = Some(found),
                    None => return false,
                },
                other => self.background = colour_of(other),
            },
            "opacity" => self.opacity = number(value),
            "borderWidth" => self.border_width = sides(value).map(|side| side.and_then(pixels_of)),
            "borderTopWidth" => self.border_width[0] = number(value),
            "borderRightWidth" => self.border_width[1] = number(value),
            "borderBottomWidth" => self.border_width[2] = number(value),
            "borderLeftWidth" => self.border_width[3] = number(value),
            "borderColor" => self.border_color = colour_of(value),
            "radius" => {
                let parts = sides_list(value);
                self.radius = match parts.len() {
                    1 => [parts[0]; 4],
                    4 => [parts[0], parts[1], parts[2], parts[3]],
                    _ => return false,
                }
            }
            "shadow" => self.shadow = shadows(value),
            "color" => self.color = colour_of(value),
            "fontSize" => self.font_size = number(value),
            "fontWeight" => {
                self.font_weight = match value {
                    Value::Number(weight) => weight.as_f64().map(|weight| weight as f32),
                    Value::String(name) => match name.as_str() {
                        "normal" => Some(400.0),
                        "medium" => Some(500.0),
                        "semibold" => Some(600.0),
                        "bold" => Some(700.0),
                        _ => return false,
                    },
                    _ => return false,
                }
            }
            "fontFamily" => {
                self.font_family = value.as_str().map(|family| match family {
                    "sans" => SharedString::from(SANS),
                    "mono" => SharedString::from(MONO),
                    other => SharedString::from(other.to_string()),
                })
            }
            "lineHeight" => self.line_height = number(value),
            "italic" => self.italic = value.as_bool(),
            "underline" => self.underline = value.as_bool(),
            "strikethrough" => self.strikethrough = value.as_bool(),
            "textAlign" => {
                self.text_align = match value.as_str() {
                    Some("left") => Some(TextAlign::Left),
                    Some("center") => Some(TextAlign::Center),
                    Some("right") => Some(TextAlign::Right),
                    _ => return false,
                }
            }
            "nowrap" => self.nowrap = value.as_bool(),
            "truncate" => self.truncate = value.as_bool(),
            "maxLines" => self.max_lines = value.as_u64().map(|lines| lines as usize),
            "uppercase" => self.uppercase = value.as_bool(),
            "cursor" => {
                self.cursor = match value.as_str() {
                    Some("default") => Some(CursorStyle::Arrow),
                    Some("pointer") => Some(CursorStyle::PointingHand),
                    Some("text") => Some(CursorStyle::IBeam),
                    Some("grab") => Some(CursorStyle::OpenHand),
                    Some("grabbing") => Some(CursorStyle::ClosedHand),
                    Some("crosshair") => Some(CursorStyle::Crosshair),
                    Some("col-resize") => Some(CursorStyle::ResizeLeftRight),
                    Some("row-resize") => Some(CursorStyle::ResizeUpDown),
                    Some("not-allowed") => Some(CursorStyle::OperationNotAllowed),
                    _ => return false,
                }
            }
            _ => return false,
        }
        true
    }

    /// Applies an inline declaration Svelte's transitions write, as CSS: they animate
    /// `opacity`, `transform` and the box's size by writing them each frame.
    pub fn apply_inline(&mut self, name: &str, value: &str) {
        let value = value.trim().trim_end_matches("!important").trim();
        let length = |value: &str| -> Option<Dimension> {
            if let Some(percent) = value.strip_suffix('%') {
                return percent.trim().parse().ok().map(Dimension::Percent);
            }
            if value == "auto" {
                return Some(Dimension::Auto);
            }
            value.trim_end_matches("px").trim().parse().ok().map(Dimension::Px)
        };
        let pixels = |value: &str| value.trim_end_matches("px").trim().parse::<f32>().ok();
        match name {
            "opacity" => self.opacity = value.parse().ok(),
            "transform" => {
                let transform = parse_transform(value);
                self.translate = Some(transform.translate);
                if transform.scale.is_some() {
                    self.scale = transform.scale;
                }
                if transform.rotate.is_some() {
                    self.rotate = transform.rotate;
                }
            }
            "width" => self.width = length(value),
            "height" => self.height = length(value),
            "min-height" => self.min_height = length(value),
            "max-height" => self.max_height = length(value),
            "overflow" => self.clip = Some(value == "hidden"),
            "padding-top" => self.padding[0] = pixels(value),
            "padding-bottom" => self.padding[2] = pixels(value),
            "margin-top" => self.margin[0] = length(value),
            "margin-bottom" => self.margin[2] = length(value),
            "border-top-width" => self.border_width[0] = pixels(value),
            "border-bottom-width" => self.border_width[2] = pixels(value),
            _ => {}
        }
    }
}

fn number(value: &Value) -> Option<f32> {
    value.as_f64().map(|number| number as f32)
}

/// `12` is pixels, `"50%"` a share of the parent, `"auto"` auto.
fn dimension(value: &Value) -> Option<Dimension> {
    match value {
        Value::Number(number) => number.as_f64().map(|pixels| Dimension::Px(pixels as f32)),
        Value::String(text) => {
            if text == "auto" {
                return Some(Dimension::Auto);
            }
            if text == "full" {
                return Some(Dimension::Percent(100.0));
            }
            text.strip_suffix('%')?.trim().parse().ok().map(Dimension::Percent)
        }
        _ => None,
    }
}

fn pixels_of(dimension: Dimension) -> Option<f32> {
    match dimension {
        Dimension::Px(pixels) => Some(pixels),
        _ => None,
    }
}

fn align(value: &Value) -> Option<Align> {
    match value.as_str()? {
        "start" => Some(Align::Start),
        "end" => Some(Align::End),
        "center" => Some(Align::Center),
        "stretch" => Some(Align::Stretch),
        "baseline" => Some(Align::Baseline),
        _ => None,
    }
}

fn sides_list(value: &Value) -> Vec<Option<f32>> {
    match value {
        Value::Array(parts) => parts.iter().map(number).collect(),
        other => vec![number(other)],
    }
}

/// One value for every side, `[vertical, horizontal]`, or `[top, right, bottom, left]`.
fn sides(value: &Value) -> Sides<Option<Dimension>> {
    let parts: Vec<Option<Dimension>> = match value {
        Value::Array(parts) => parts.iter().map(dimension).collect(),
        other => vec![dimension(other)],
    };
    match parts.as_slice() {
        [all] => [*all; 4],
        [vertical, horizontal] => [*vertical, *horizontal, *vertical, *horizontal],
        [top, right, bottom, left] => [*top, *right, *bottom, *left],
        _ => [None; 4],
    }
}

fn transition(value: &Value) -> Option<Transition> {
    if let Some(duration) = number(value) {
        return Some(Transition { duration_ms: duration, delay_ms: 0.0, easing: Easing::EASE });
    }
    let fields = value.as_object()?;
    let easing = match fields.get("easing").and_then(Value::as_str) {
        Some("linear") => Easing::LINEAR,
        Some("ease-in") => Easing(0.42, 0.0, 1.0, 1.0),
        Some("ease-out") => Easing(0.0, 0.0, 0.58, 1.0),
        Some("ease-in-out") => Easing(0.42, 0.0, 0.58, 1.0),
        _ => Easing::EASE,
    };
    Some(Transition {
        duration_ms: fields.get("duration").and_then(number).unwrap_or(150.0),
        delay_ms: fields.get("delay").and_then(number).unwrap_or(0.0),
        easing,
    })
}

fn shadows(value: &Value) -> Option<Vec<Shadow>> {
    let list: Vec<&Value> = match value {
        Value::Array(items) => items.iter().collect(),
        Value::Null => return Some(Vec::new()),
        other => vec![other],
    };
    list.into_iter()
        .map(|item| {
            let fields = item.as_object()?;
            let read = |name: &str| fields.get(name).and_then(number).unwrap_or(0.0);
            Some(Shadow {
                x: read("x"),
                y: read("y"),
                blur: read("blur"),
                spread: read("spread"),
                color: fields.get("color").and_then(colour_of).unwrap_or(Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.3 }),
            })
        })
        .collect()
}

/// The most stops a gradient has; the host's renderer takes no more.
const MOST_STOPS: usize = 4;

/// `{ linear: { angle?, stops } }` or `{ radial: { center?, radius?, stops } }`, where `stops` is
/// two to four stops, each a colour or `[colour, position]`. A stop without a position is
/// spread evenly between its neighbours' ends, the first at 0 and the last at 1.
fn gradient(value: &Value) -> Option<Gradient> {
    if let Some(linear) = value.get("linear").and_then(Value::as_object) {
        let angle = linear.get("angle").and_then(number).unwrap_or(180.0);
        return Some(Gradient::Linear { angle, stops: stops(linear.get("stops")?)? });
    }
    let radial = value.get("radial")?.as_object()?;
    let center = pair(radial.get("center"), (0.5, 0.5));
    let radius = pair(radial.get("radius"), (0.5, 0.5));
    Some(Gradient::Radial { center, radius, stops: stops(radial.get("stops")?)? })
}

/// `[x, y]`, a single number for both, or `default`.
fn pair(value: Option<&Value>, default: (f32, f32)) -> (f32, f32) {
    match value {
        Some(Value::Array(parts)) => match (parts.first().and_then(number), parts.get(1).and_then(number)) {
            (Some(x), Some(y)) => (x, y),
            _ => default,
        },
        Some(other) => number(other).map_or(default, |both| (both, both)),
        None => default,
    }
}

fn stops(value: &Value) -> Option<Vec<Stop>> {
    let listed = value.as_array()?;
    if listed.len() < 2 || listed.len() > MOST_STOPS {
        return None;
    }
    let last = listed.len() - 1;
    listed.iter().enumerate().map(|(index, item)| stop(item, index as f32 / last as f32)).collect()
}

fn stop(value: &Value, default_position: f32) -> Option<(Rgba, f32)> {
    match value {
        Value::Array(parts) => {
            Some((colour_of(parts.first()?)?, parts.get(1).and_then(number).unwrap_or(default_position)))
        }
        other => Some((colour_of(other)?, default_position)),
    }
}

fn colour_of(value: &Value) -> Option<Rgba> {
    parse_colour(value.as_str()?)
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(…)`, `rgba(…)` or `transparent`.
pub fn parse_colour(text: &str) -> Option<Rgba> {
    let text = text.trim();
    if text == "transparent" {
        return Some(Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });
    }
    if let Some(hex) = text.strip_prefix('#') {
        let digit = |index: usize| u8::from_str_radix(&hex[index..index + 1], 16).ok().map(|value| value * 17);
        let pair = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
        let channels = match hex.len() {
            3 => [digit(0)?, digit(1)?, digit(2)?, 255],
            4 => [digit(0)?, digit(1)?, digit(2)?, digit(3)?],
            6 => [pair(0)?, pair(2)?, pair(4)?, 255],
            8 => [pair(0)?, pair(2)?, pair(4)?, pair(6)?],
            _ => return None,
        };
        let [r, g, b, a] = channels.map(|channel| channel as f32 / 255.0);
        return Some(Rgba { r, g, b, a });
    }
    let inner = text.strip_prefix("rgba(").or_else(|| text.strip_prefix("rgb("))?.strip_suffix(')')?;
    let parts: Vec<f32> = inner
        .split([',', ' ', '/'])
        .filter(|part| !part.is_empty())
        .map(|part| part.trim().parse::<f32>().ok())
        .collect::<Option<_>>()?;
    let alpha = parts.get(3).copied().unwrap_or(1.0);
    Some(Rgba { r: *parts.first()? / 255.0, g: *parts.get(1)? / 255.0, b: *parts.get(2)? / 255.0, a: alpha })
}

/// What a CSS `transform` says that the host draws.
#[derive(Debug, Default, PartialEq)]
struct Transform {
    translate: (f32, f32),
    scale: Option<f32>,
    rotate: Option<f32>,
}

/// `translate(x, y)`, `translateX(x)`, `translateY(y)`, `scale(s)` and `rotate(Ndeg)` from a CSS
/// transform. Scales multiply, as the functions would compose, and a scale of two numbers takes the first.
fn parse_transform(value: &str) -> Transform {
    let mut transform = Transform::default();
    let mut rest = value;
    while let Some(open) = rest.find('(') {
        let name = rest[..open].trim();
        let Some(close) = rest[open..].find(')') else {
            break;
        };
        let arguments: Vec<f32> = rest[open + 1..open + close]
            .split(',')
            .map(|part| part.trim().trim_end_matches("px").trim_end_matches("deg").parse().unwrap_or(0.0))
            .collect();
        let first = arguments.first().copied().unwrap_or(0.0);
        match name {
            "translate" => {
                transform.translate.0 += first;
                transform.translate.1 += arguments.get(1).copied().unwrap_or(0.0);
            }
            "translateX" => transform.translate.0 += first,
            "translateY" => transform.translate.1 += first,
            "scale" => transform.scale = Some(transform.scale.unwrap_or(1.0) * first),
            "rotate" => transform.rotate = Some(transform.rotate.unwrap_or(0.0) + first),
            _ => {}
        }
        rest = &rest[open + close + 1..];
    }
    transform
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn style_of(value: Value) -> Style {
        let mut style = Style::default();
        for (name, value) in value.as_object().expect("an object") {
            assert!(style.set(name, value), "{name} is understood");
        }
        style
    }

    #[test]
    fn a_css_transform_gives_translation_scale_and_rotation() {
        let transform = parse_transform("translate(4px, -2px) scale(0.5) rotate(90deg)");
        assert_eq!(transform, Transform { translate: (4.0, -2.0), scale: Some(0.5), rotate: Some(90.0) });
        assert_eq!(parse_transform("translateX(3px)").scale, None);
    }

    #[test]
    fn scale_and_rotate_are_style_properties() {
        let style = style_of(json!({ "scale": 1.25, "rotate": 45 }));
        assert_eq!((style.scale, style.rotate), (Some(1.25), Some(45.0)));
    }

    #[test]
    fn gradient_reads_angle_and_stops() {
        let style =
            style_of(json!({ "background": { "linear": { "angle": 135, "stops": [["#ff0000", 0.25], "#0000ff"] } } }));
        let Some(Gradient::Linear { angle, stops }) = style.gradient else {
            panic!("a linear gradient");
        };
        assert_eq!(angle, 135.0);
        assert_eq!(stops[0], (Rgba { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }, 0.25));
        assert_eq!(stops[1], (Rgba { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }, 1.0));
        assert_eq!(style.background, None);
    }

    #[test]
    fn gradient_points_down_by_default() {
        let style = style_of(json!({ "background": { "linear": { "stops": ["#000", "#fff"] } } }));
        assert!(matches!(style.gradient, Some(Gradient::Linear { angle, .. }) if angle == 180.0));
    }

    #[test]
    fn gradient_needs_two_to_four_stops() {
        let mut style = Style::default();
        let one = json!({ "linear": { "stops": ["#000"] } });
        let five = json!({ "linear": { "stops": ["#000", "#222", "#444", "#888", "#fff"] } });
        assert!(!style.set("background", &one));
        assert!(!style.set("background", &five));
        assert_eq!(style.gradient, None);
    }

    #[test]
    fn stops_without_a_position_are_spread_evenly() {
        let style =
            style_of(json!({ "background": { "linear": { "stops": ["#000", ["#888", 0.9], "#aaa", "#fff"] } } }));
        let Some(Gradient::Linear { stops, .. }) = style.gradient else {
            panic!("a linear gradient");
        };
        let positions: Vec<f32> = stops.iter().map(|(_, at)| *at).collect();
        assert_eq!(positions, vec![0.0, 0.9, 2.0 / 3.0, 1.0]);
    }

    #[test]
    fn a_radial_gradient_reads_centre_radius_and_stops() {
        let style = style_of(json!({
            "background": { "radial": { "center": [1, 0.25], "radius": 0.8, "stops": ["#f00", "#00f00000"] } }
        }));
        let Some(Gradient::Radial { center, radius, stops }) = style.gradient else {
            panic!("a radial gradient");
        };
        assert_eq!((center, radius), ((1.0, 0.25), (0.8, 0.8)));
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[1].0.a, 0.0);
    }

    #[test]
    fn a_radial_gradient_is_centred_by_default() {
        let style = style_of(json!({ "background": { "radial": { "stops": ["#000", "#fff"] } } }));
        assert!(matches!(style.gradient, Some(Gradient::Radial { center: (0.5, 0.5), radius: (0.5, 0.5), .. })));
    }

    #[test]
    fn a_colour_patch_replaces_a_gradient_and_back() {
        let mut base = style_of(json!({ "background": { "linear": { "stops": ["#000", "#fff"] } } }));
        base.overlay(&style_of(json!({ "background": "#123456" })));
        assert!(base.background.is_some());
        assert_eq!(base.gradient, None);
        base.overlay(&style_of(json!({ "background": { "linear": { "stops": ["#000", "#fff"] } } })));
        assert!(base.gradient.is_some());
        assert_eq!(base.background, None);
    }
}
