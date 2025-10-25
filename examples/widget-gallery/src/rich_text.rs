use std::ops::Range;

use floem::{
    peniko::color::palette,
    text::{Attrs, AttrsList, Style, TextLayout},
    views::{rich_text, scroll, v_stack, Decorators, RichTextExt},
    IntoView,
};

pub fn rich_text_view() -> impl IntoView {
    let builder =
        "This".red().italic() + " is rich text".blue() + "\nTest value: " + 5.to_string().green();

    let text = "
    // floem is an UI library, homepage: https://github.com/lapce/floem
    fn main() {
        println!(\"Hello World!\");
    }";
    scroll({
        v_stack((
            rich_text(move || {
                let attrs = Attrs::new().color(palette::css::BLACK);

                let mut attrs_list = AttrsList::new(attrs);

                attrs_list.add_span(
                    Range { start: 6, end: 67 },
                    Attrs::new().color(palette::css::GRAY).style(Style::Italic),
                );

                attrs_list.add_span(
                    Range { start: 42, end: 72 },
                    Attrs::new().color(palette::css::BLUE),
                );

                attrs_list.add_span(
                    Range { start: 77, end: 79 },
                    Attrs::new().color(palette::css::PURPLE),
                );

                attrs_list.add_span(
                    Range { start: 80, end: 84 },
                    Attrs::new().color(palette::css::SKY_BLUE),
                );

                attrs_list.add_span(
                    Range { start: 84, end: 86 },
                    Attrs::new().color(palette::css::GOLDENROD),
                );

                attrs_list.add_span(
                    Range {
                        start: 97,
                        end: 104,
                    },
                    Attrs::new().color(palette::css::GOLD),
                );

                attrs_list.add_span(
                    Range {
                        start: 104,
                        end: 105,
                    },
                    Attrs::new().color(palette::css::PURPLE),
                );

                attrs_list.add_span(
                    Range {
                        start: 106,
                        end: 119,
                    },
                    Attrs::new().color(palette::css::DARK_GREEN),
                );

                attrs_list.add_span(
                    Range {
                        start: 119,
                        end: 120,
                    },
                    Attrs::new().color(palette::css::PURPLE),
                );

                attrs_list.add_span(
                    Range {
                        start: 120,
                        end: 121,
                    },
                    Attrs::new().color(palette::css::GRAY),
                );

                let mut text_layout = TextLayout::new();
                text_layout.set_text(text, attrs_list, None);
                text_layout
            }),
            builder.style(|s| s.padding_left(15)),
        ))
        .style(|s| s.gap(20))
    })
}
