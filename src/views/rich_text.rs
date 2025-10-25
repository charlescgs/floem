#![deny(missing_docs)]
use std::any::Any;

use floem_reactive::create_effect;
use floem_renderer::{
    Renderer,
    text::{Attrs, AttrsList, AttrsOwned, TextLayout},
};
use peniko::{
    Color,
    color::palette,
    kurbo::{Point, Rect},
};
use smallvec::{SmallVec, smallvec};
use taffy::tree::NodeId;

use crate::{
    IntoView,
    context::UpdateCx,
    id::ViewId,
    style::{Style, TextOverflow},
    unit::PxPct,
    view::View,
};

/// A struct holding data needed to construct [RichText] widget.
///
/// For examples see [rich_text].
pub struct RichText {
    id: ViewId,
    text_layout: TextLayout,
    text_node: Option<NodeId>,
    text_overflow: TextOverflow,
    available_width: Option<f32>,
    available_text_layout: Option<TextLayout>,
}

/// Constructs rich text widget.
///
/// ```rust
/// # use core::ops::Range;
/// # use floem::prelude::palette::css;
/// # use floem::text::{Attrs, AttrsList, Style, TextLayout, Weight, FamilyOwned};
/// # use floem::views::{rich_text, stack};
///
/// let text = "This is rich text";
/// let rich = move || {
///     // Construct attributes list from attributes
///     let mut attrs_list = AttrsList::new(Attrs::new());
///     // Add as many spans as you need...
///     attrs_list.add_span(
///         Range { start: 0, end: 17 },
///         // ...and style them
///         Attrs::new().color(css::DARK_GRAY).style(Style::Italic),
///     );
///     attrs_list.add_span(
///         Range { start: 5, end: 6 },
///         Attrs::new().color(css::BLUE).font_size(25.),
///     );
///     attrs_list.add_span(
///         Range { start: 8, end: 11 },
///         Attrs::new().color(css::PURPLE).weight(Weight::EXTRA_BOLD),
///     );
///     attrs_list.add_span(
///         Range { start: 12, end: 17 },
///         Attrs::new().color(css::RED).family(&[FamilyOwned::Monospace]),
///     );
///     // Construct text layout
///     let mut text_layout = TextLayout::new();
///     // Set text with attributes and optional align
///     text_layout.set_text(text, attrs_list, None);
///     text_layout
/// };
/// stack((
///     rich_text(move || rich()),
/// ));
/// ```
pub fn rich_text(text_layout: impl Fn() -> TextLayout + 'static) -> RichText {
    let id = ViewId::new();
    let text = text_layout();
    create_effect(move |_| {
        let new_text_layout = text_layout();
        id.update_state(new_text_layout);
    });
    RichText {
        id,
        text_layout: text,
        text_node: None,
        text_overflow: TextOverflow::Wrap,
        available_width: None,
        available_text_layout: None,
    }
}

impl View for RichText {
    fn id(&self) -> ViewId {
        self.id
    }

    fn debug_name(&self) -> std::borrow::Cow<'static, str> {
        format!(
            "RichText: {:?}",
            self.text_layout
                .lines()
                .iter()
                .map(|text| text.text())
                .collect::<String>()
        )
        .into()
    }

    fn update(&mut self, _cx: &mut UpdateCx, state: Box<dyn Any>) {
        if let Ok(state) = state.downcast() {
            self.text_layout = *state;
            self.available_width = None;
            self.available_text_layout = None;
            self.id.request_layout();
        }
    }

    fn layout(&mut self, cx: &mut crate::context::LayoutCx) -> taffy::tree::NodeId {
        cx.layout_node(self.id(), true, |_cx| {
            let size = self.text_layout.size();
            let width = size.width as f32;
            let mut height = size.height as f32;

            if let Some(t) = self.available_text_layout.as_ref() {
                height = height.max(t.size().height as f32);
            }

            if self.text_node.is_none() {
                self.text_node = Some(
                    self.id
                        .taffy()
                        .borrow_mut()
                        .new_leaf(taffy::style::Style::DEFAULT)
                        .unwrap(),
                );
            }
            let text_node = self.text_node.unwrap();

            let style = Style::new().width(width).height(height).to_taffy_style();
            let _ = self.id.taffy().borrow_mut().set_style(text_node, style);
            vec![text_node]
        })
    }

    fn compute_layout(&mut self, _cx: &mut crate::context::ComputeLayoutCx) -> Option<Rect> {
        let layout = self.id.get_layout().unwrap_or_default();
        let view_state = self.id.state();
        let (padding_left, padding_right) = {
            let view_state = view_state.borrow();
            let style = view_state.combined_style.builtin();
            let padding_left = match style.padding_left() {
                PxPct::Px(padding) => padding as f32,
                PxPct::Pct(pct) => pct as f32 * layout.size.width,
            };
            let padding_right = match style.padding_right() {
                PxPct::Px(padding) => padding as f32,
                PxPct::Pct(pct) => pct as f32 * layout.size.width,
            };
            self.text_overflow = style.text_overflow();
            (padding_left, padding_right)
        };

        let padding = padding_left + padding_right;
        let width = self.text_layout.size().width as f32;
        let available_width = layout.size.width - padding;
        if self.text_overflow == TextOverflow::Wrap {
            if width > available_width {
                if self.available_width != Some(available_width) {
                    let mut text_layout = self.text_layout.clone();
                    text_layout.set_size(available_width, f32::MAX);
                    self.available_text_layout = Some(text_layout);
                    self.available_width = Some(available_width);
                    self.id.request_layout();
                }
            } else {
                if self.available_text_layout.is_some() {
                    self.id.request_layout();
                }
                self.available_text_layout = None;
                self.available_width = None;
            }
        }

        None
    }

    fn paint(&mut self, cx: &mut crate::context::PaintCx) {
        let text_node = self.text_node.unwrap();
        let location = self
            .id
            .taffy()
            .borrow_mut()
            .layout(text_node)
            .cloned()
            .unwrap_or_default()
            .location;
        let point = Point::new(location.x as f64, location.y as f64);
        if let Some(text_layout) = self.available_text_layout.as_ref() {
            cx.draw_text(text_layout, point);
        } else {
            cx.draw_text(&self.text_layout, point);
        }
    }
}

/// Reference of rich text span with its attributes.
#[derive(Clone, Debug)]
pub struct RichSpan<'a> {
    text: &'a str,
    attrs: Attrs<'a>,
}
#[allow(clippy::wrong_self_convention)]
impl<'a> RichSpan<'a> {
    fn to_owned(self) -> RichSpanOwned {
        let len = self.text.len();
        RichSpanOwned {
            text: self.text.to_string(),
            spans: smallvec::smallvec![(0..len, AttrsOwned::new(self.attrs))],
        }
    }

    /// Set rich span color in sRBG format.
    pub fn color(mut self, color: Color) -> Self {
        self.attrs = self.attrs.color(color);
        self
    }

    /// Set rich span family:
    /// - Serif
    /// - SansSerif
    /// - Cursive
    /// - Fantasy
    /// - Monospace
    /// - Named
    pub fn family(mut self, family: &'a [floem_renderer::text::FamilyOwned]) -> RichSpan<'a> {
        self.attrs = self.attrs.family(family);
        self
    }

    /// Set rich span stretch:
    /// - Ultra Condensed
    /// - Extra Condensed
    /// - Condensed
    /// - Semi Condensed
    /// - Normal
    /// - Semi Expanded
    /// - Expanded
    /// - Extra Expanded
    /// - Ultra Expanded
    pub fn stretch(mut self, stretch: floem_renderer::text::Stretch) -> RichSpan<'a> {
        self.attrs = self.attrs.stretch(stretch);
        self
    }

    /// Set rich span text style:
    /// - Normal
    /// - Italic
    /// - Oblique
    pub fn text_style(mut self, style: floem_renderer::text::Style) -> RichSpan<'a> {
        self.attrs = self.attrs.style(style);
        self
    }

    /// Set rich span text weight (glyphs weight the font, their degree of blackness
    /// or stroke thickness).
    ///
    /// Available constants:
    /// - THIN
    /// - EXTRA_LIGHT
    /// - LIGHT
    /// - NORMAL
    /// - MEDIUM
    /// - SEMIBOLD
    /// - BOLD
    /// - EXTRA_BOLD
    /// - BLACK
    pub fn weight(mut self, weight: floem_renderer::text::Weight) -> RichSpan<'a> {
        self.attrs = self.attrs.weight(weight);
        self
    }

    /// Set rich span line height.
    pub fn line_height(
        mut self,
        line_height: floem_renderer::text::LineHeightValue,
    ) -> RichSpan<'a> {
        self.attrs = self.attrs.line_height(line_height);
        self
    }

    /// Set rich span font size.
    pub fn font_size(mut self, font_size: f32) -> RichSpan<'a> {
        self.attrs = self.attrs.font_size(font_size);
        self
    }

    /// Set rich span weight as a raw number.
    pub fn raw_weight(mut self, weight: u16) -> RichSpan<'a> {
        self.attrs = self.attrs.raw_weight(weight);
        self
    }
}

/// Owned rich text span with attributes.
#[derive(Clone, Debug)]
pub struct RichSpanOwned {
    text: String,
    spans: SmallVec<[(std::ops::Range<usize>, AttrsOwned); 3]>,
}

impl IntoView for RichSpanOwned {
    type V = RichText;

    fn into_view(self) -> Self::V {
        let mut layout = TextLayout::new();
        let mut attrs_list = AttrsList::new(Attrs::new().color(palette::css::BLACK));
        for span in self.spans {
            attrs_list.add_span(span.0, span.1.as_attrs());
        }

        layout.set_text(&self.text, attrs_list, None);
        rich_text(move || layout.clone())
    }
}

impl IntoView for RichSpan<'_> {
    type V = RichText;

    fn into_view(self) -> Self::V {
        self.to_owned().into_view()
    }
}

impl<'a, S> std::ops::Add<S> for RichSpan<'a>
where
    RichSpan<'a>: From<S>,
{
    type Output = RichSpanOwned;

    fn add(self, rhs: S) -> Self::Output {
        let self_len = self.text.len();
        let rhs: RichSpan = rhs.into();
        let rhs_len = rhs.text.len();
        RichSpanOwned {
            text: self.text.to_string() + rhs.text,
            spans: smallvec![
                (0..self_len, AttrsOwned::new(self.attrs)),
                (self_len..self_len + rhs_len, AttrsOwned::new(rhs.attrs)),
            ],
        }
    }
}

impl<'a> std::ops::Add<&'a str> for RichSpan<'a> {
    type Output = RichSpanOwned;

    fn add(self, rhs: &'a str) -> Self::Output {
        let self_len = self.text.len();
        let rhs_len = rhs.len();
        RichSpanOwned {
            text: self.text.to_string() + rhs,
            spans: smallvec![
                (0..self_len, AttrsOwned::new(self.attrs)),
                (
                    self_len..self_len + rhs_len,
                    AttrsOwned::new(Attrs::new().color(palette::css::BLACK))
                ),
            ],
        }
    }
}

impl std::ops::Add<String> for RichSpan<'_> {
    type Output = RichSpanOwned;

    fn add(self, rhs: String) -> Self::Output {
        let self_len = self.text.len();
        let rhs_len = rhs.len();
        RichSpanOwned {
            text: self.text.to_string() + &rhs,
            spans: smallvec![
                (0..self_len, AttrsOwned::new(self.attrs)),
                (
                    self_len..self_len + rhs_len,
                    AttrsOwned::new(Attrs::new().color(palette::css::BLACK))
                ),
            ],
        }
    }
}

impl<'a, S> std::ops::Add<S> for RichSpanOwned
where
    RichSpan<'a>: From<S>,
{
    type Output = Self;

    fn add(mut self, rhs: S) -> Self::Output {
        let rhs: RichSpan = rhs.into();
        let self_len = self.text.len();
        let new_text = self.text + rhs.text;
        self.spans
            .push((self_len..new_text.len(), AttrsOwned::new(rhs.attrs)));
        Self {
            text: new_text,
            spans: self.spans,
        }
    }
}

impl std::ops::Add<&str> for RichSpanOwned {
    type Output = RichSpanOwned;

    fn add(mut self, rhs: &str) -> Self::Output {
        let self_len = self.text.len();
        let new_text = self.text + rhs;
        self.spans.push((
            self_len..new_text.len(),
            AttrsOwned::new(Attrs::new().color(palette::css::BLACK)),
        ));
        Self {
            text: new_text,
            spans: self.spans,
        }
    }
}

impl std::ops::Add<String> for RichSpanOwned {
    type Output = RichSpanOwned;

    fn add(mut self, rhs: String) -> Self::Output {
        let self_len = self.text.len();
        let new_text = self.text + &rhs;
        self.spans.push((
            self_len..new_text.len(),
            AttrsOwned::new(Attrs::new().color(palette::css::BLACK)),
        ));
        Self {
            text: new_text,
            spans: self.spans,
        }
    }
}

impl std::ops::Add for RichSpanOwned {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        let self_len = self.text.len();
        self.spans.extend(
            rhs.spans
                .into_iter()
                .map(|span| ((span.0.start + self_len)..(span.0.end + self_len), span.1)),
        );
        Self {
            text: self.text + &rhs.text,
            spans: self.spans,
        }
    }
}

/// Contains rich text style methods.
///
/// ### Example
/// ```rust
/// use floem::views::RichTextExt;
///
/// let rich_text_builder =
///     "This".red().italic()
///     + " is rich text".blue()
///     + "\nTest value: "
///     + 5.to_string().green();
/// ```
pub trait RichTextExt<'a>
where
    Self: Sized,
    RichSpan<'a>: From<Self>,
{
    /// Set rich span color in sRBG format.
    fn color(self, color: Color) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.color(color)
    }

    /// Set rich span color to [palette::css::RED].
    fn red(self) -> RichSpan<'a> {
        self.color(palette::css::RED)
    }

    /// Set rich span color to [palette::css::BLUE].
    fn blue(self) -> RichSpan<'a> {
        self.color(palette::css::BLUE)
    }

    /// Set rich span color to [palette::css::GREEN].
    fn green(self) -> RichSpan<'a> {
        self.color(palette::css::GREEN)
    }

    /// Set rich span color to [palette::css::YELLOW].
    fn yellow(self) -> RichSpan<'a> {
        self.color(palette::css::YELLOW)
    }

    /// Set rich span color to [palette::css::BLACK].
    fn black(self) -> RichSpan<'a> {
        self.color(palette::css::BLACK)
    }

    /// Set rich span color to [palette::css::WHITE].
    fn white(self) -> RichSpan<'a> {
        self.color(palette::css::WHITE)
    }

    /// Set rich span color to [palette::css::GRAY].
    fn gray(self) -> RichSpan<'a> {
        self.color(palette::css::GRAY)
    }

    /// Set rich span color to [palette::css::CYAN].
    fn cyan(self) -> RichSpan<'a> {
        self.color(palette::css::CYAN)
    }

    /// Set rich span color to [palette::css::MAGENTA].
    fn magenta(self) -> RichSpan<'a> {
        self.color(palette::css::MAGENTA)
    }

    /// Set rich span color to [palette::css::ORANGE].
    fn orange(self) -> RichSpan<'a> {
        self.color(palette::css::ORANGE)
    }

    /// Set rich span color to [palette::css::PURPLE].
    fn purple(self) -> RichSpan<'a> {
        self.color(palette::css::PURPLE)
    }

    /// Set rich span color to [palette::css::PINK].
    fn pink(self) -> RichSpan<'a> {
        self.color(palette::css::PINK)
    }

    /// Set rich span font family:
    /// - Serif
    /// - SansSerif
    /// - Cursive
    /// - Fantasy
    /// - Monospace
    /// - Named
    fn family(self, family: &'a [crate::text::FamilyOwned]) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.family(family)
    }

    /// Set rich span stretch:
    /// - Ultra Condensed
    /// - Extra Condensed
    /// - Condensed
    /// - Semi Condensed
    /// - Normal
    /// - Semi Expanded
    /// - Expanded
    /// - Extra Expanded
    /// - Ultra Expanded
    fn stretch(self, stretch: crate::text::Stretch) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.stretch(stretch)
    }

    /// Set rich span text style:
    /// - Normal
    /// - Italic
    /// - Oblique
    fn text_style(self, style: crate::text::Style) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.text_style(style)
    }

    /// Set rich span text style to `italic` (cursive).
    fn italic(self) -> RichSpan<'a> {
        self.text_style(crate::text::Style::Italic)
    }

    /// Set rich span text style to `oblique`
    /// (typically-sloped version of the regular face).
    fn oblique(self) -> RichSpan<'a> {
        self.text_style(crate::text::Style::Oblique)
    }

    /// Set rich span text weight (glyphs weight the font, their degree of blackness
    /// or stroke thickness).
    ///
    /// Available constants:
    /// - THIN
    /// - EXTRA_LIGHT
    /// - LIGHT
    /// - NORMAL
    /// - MEDIUM
    /// - SEMIBOLD
    /// - BOLD
    /// - EXTRA_BOLD
    /// - BLACK
    fn weight(self, weight: crate::text::Weight) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.weight(weight)
    }

    /// Set rich span text weight to `THIN`.
    fn thin(self) -> RichSpan<'a> {
        self.weight(crate::text::Weight::THIN)
    }

    /// Set rich span text weight to `EXTRA_LIGHT`.
    fn extra_light(self) -> RichSpan<'a> {
        self.weight(crate::text::Weight::EXTRA_LIGHT)
    }

    /// Set rich span text weight to `LIGHT`.
    fn light(self) -> RichSpan<'a> {
        self.weight(crate::text::Weight::LIGHT)
    }

    /// Set rich span text weight to `MEDIUM`.
    fn medium(self) -> RichSpan<'a> {
        self.weight(crate::text::Weight::MEDIUM)
    }

    /// Set rich span text weight to `SEMIBOLD`.
    fn semibold(self) -> RichSpan<'a> {
        self.weight(crate::text::Weight::SEMIBOLD)
    }

    /// Set rich span text weight to `BOLD`.
    fn bold(self) -> RichSpan<'a> {
        self.weight(crate::text::Weight::BOLD)
    }

    /// Set rich span text weight to `EXTRA_BOLD`.
    fn extra_bold(self) -> RichSpan<'a> {
        self.weight(crate::text::Weight::EXTRA_BOLD)
    }

    /// Set rich span weight as a raw number.
    fn raw_weight(self, weight: u16) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.raw_weight(weight)
    }

    /// Set rich span font size.
    fn font_size(self, font_size: f32) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.font_size(font_size)
    }

    /// Set rich span line height.
    fn line_height(self, line_height: crate::text::LineHeightValue) -> RichSpan<'a> {
        let span: RichSpan = self.into();
        span.line_height(line_height)
    }
}

impl<'a, S> RichTextExt<'a> for S
where
    S: AsRef<str>,
    RichSpan<'a>: From<S>,
{
}

impl<'a, S: AsRef<str> + 'a> From<&'a S> for RichSpan<'a> {
    fn from(value: &'a S) -> Self {
        RichSpan {
            text: value.as_ref(),
            attrs: Attrs::new().color(palette::css::BLACK),
        }
    }
}

impl<'a> RichTextExt<'a> for RichSpan<'a> {}
