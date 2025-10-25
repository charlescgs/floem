#![allow(unused)]
use floem::prelude::palette::css::AQUAMARINE;
use floem::prelude::palette::css::LIGHT_CYAN;
use floem::prelude::palette::css::TRANSPARENT;
use floem::prelude::palette::css::YELLOW;
use floem::prelude::*;
use floem::reactive::create_effect;
use floem::style::BoxShadow;
use floem::style::Style;
use floem::style::StyleSelector;
use floem::style::Transition;
use floem::style_class;
use floem::window::Theme;

use crate::form;
use crate::form::form_item;
use crate::tabs_dark::dark_theme;

#[derive(Clone)]
struct TabContent {
    idx: usize,
    name: String,
}

impl TabContent {
    fn new(tabs_count: usize) -> Self {
        Self {
            idx: tabs_count,
            name: format!("Tab with index"),
        }
    }
}

#[derive(Clone)]
enum Action {
    Add,
    Remove,
    None,
}

pub fn tab_view() -> impl IntoView {
    
    
    let tabs = RwSignal::new(vec![]);
    let active_tab = RwSignal::new(None::<usize>);
    let tab_action = RwSignal::new(Action::None);
    let theme = RwSignal::new(Theme::Light);

    let ms = Style::new()
        .class(NewButtonClass, |s| s.apply(button_class()))
        .class(NewListItemClass, |s| s.apply(list_item_class()))
        .dark_mode(move |s| match theme.get() {
            Theme::Light => s,
            Theme::Dark => s.apply(dark_theme())
        });

    create_effect(move |_| match tab_action.get() {
        Action::Add => {
            tabs.update(|tabs| tabs.push(TabContent::new(tabs.len())));
        }
        Action::Remove => {
            tabs.update(|tabs| {
                tabs.pop();
            });
        }
        Action::None => (),
    });

    let tabs_view = stack((dyn_stack(
        move || tabs.get(),
        |tab| tab.idx,
        move |tab| {
            tab_side_item(tab.clone(), active_tab).on_click_stop(move |_| {
                active_tab.update(|a| {
                    *a = Some(tab.idx);
                });
            })
        },
    )
    .style(|s| s.flex_col().width_full().row_gap(5.))
    .scroll()
    .on_click_stop(move |_| {
        if active_tab.with_untracked(|act| act.is_some()) {
            active_tab.set(None)
        }
    })
    .style(|s| s.size_full().padding(5.).padding_right(7.))
    .scroll_style(|s| s.handle_thickness(6.).shrink_to_fit()),))
    .style(|s| {
        s.width(120.)
            .min_width(120.)
            .height_full()
            .border_right(1.)
            .border_color(BG_DARK)
    });

    let tabs_content_view = stack((
        tab(
            move || active_tab.get(),
            move || tabs.get(),
            |tab| tab.idx,
            move |tab| show_tab_content(tab),
        ).style(|s| s.size_full()),
    ))
    .style(|s| s.size_full());

    v_stack((
        h_stack((
            button("add tab")
                .action(move || {
                    tab_action.update(|a| {
                        *a = Action::Add;
                    })
                })
                .style(|s| s.apply(button_class().apply_class(NewButtonClass))),
            button("remove tab")
                .action(move || {
                    tab_action.update(|a| {
                        *a = Action::Remove;
                    })
                })
                .style(|s| s.apply(button_class().apply_class(NewButtonClass))),
            button("toggle_theme")
                .action(move || {
                    theme.update(|t| *t = match t {
                        Theme::Light => Theme::Dark,
                        Theme::Dark => Theme::Light
                    });
                })
                .style(|s| s.apply(button_class().apply_class(NewButtonClass))),
        ))
        .style(|s| {
            s.height(40.px())
                .width_full()
                .border_bottom(1.)
                .border_color(BG_DARK)
                .padding(5.)
                .col_gap(5.)
                .items_center()
        }),
        stack((tabs_view, tabs_content_view))
            .style(|s| s.height(400.px()).width(500.px())),
    ))
    .style(|s| s.size_full()
        .border(1.)
        .background(BG)
        .border(0.)
        
        .border_radius(8.)
        .border_top(0.5)
        .border_top_color(BG_LIGHT)
        .border_bottom(0.5)
        .border_bottom_color(TEXT_MUTED.multiply_alpha(0.7))

        .box_shadow_color(BORDER)
        .box_shadow_spread(2.)
        .box_shadow_blur(4.)

        .box_shadow_top_offset(-7.)
        .box_shadow_bottom_offset(3.)
        .box_shadow_right_offset(-3.)
        .box_shadow_left_offset(-3.)
    )
    .container()
    .style(|s| s.size_full().padding(10.).background(BG_DARK))
}

fn show_tab_content(tab: TabContent) -> impl IntoView {
    stack((
        // label(move || format!("{} is now active!", tab.name))
        //     .style(|s| s.font_size(18.).color(TEXT)),
        empty().style(|s| s
            .size(100.px(), 100.px())
            .border_radius(6.)
            .background(HIGHLIGHT)
            .box_shadow_color(BG_DARK)
            .box_shadow_top_offset(-15.)
            .box_shadow_bottom_offset(4.)
            .box_shadow_right_offset(-6.)
            .box_shadow_left_offset(-6.)
            .box_shadow_spread(5.)
            .box_shadow_blur(4.)
        ),
        empty().style(|s|s
            .size(100.px(), 100.px())
            .border_radius(6.)
            .background(HIGHLIGHT)

            .border_top(1.)
            .border_top_color(HIGHLIGHT)

            .apply_box_shadow(BoxShadow::new()
                .color(BORDER.multiply_alpha(0.55))
                .top_offset(-13.)
                .bottom_offset(0.4)
                .right_offset(-4.)
                .left_offset(-4.)
                .blur_radius(2.)
                .spread(1.5)
            )
        ),
        stack((
            v_stack((
                label(move || format!("{}", tab.name)).style(|s| s
                    .font_size(13.)
                    .font_bold()
                    .color(TEXT)),
                label(move || format!("{}", tab.idx)).style(|s| s
                    .font_size(16.)
                    .font_bold()
                    .color(TEXT)),
                label(move || "is now active").style(|s| s
                    .font_size(11.)
                    .color(TEXT_MUTED))
            )).style(|s| s.size_full().items_center().justify_center()),
        )).style(|s|s
            .size(100.px(), 100.px())
            .border_radius(6.)
            .background(HIGHLIGHT)

            .border_top(0.6)
            .border_top_color(HIGHLIGHT)
            
            .box_shadow_color(BG_DARK)
            .box_shadow_top_offset(-15.)
            .box_shadow_bottom_offset(4.)
            .box_shadow_right_offset(-6.)
            .box_shadow_left_offset(-6.)
            .box_shadow_spread(5.)
            .box_shadow_blur(4.)
            .apply_box_shadow(BoxShadow::new()
                .color(BORDER.multiply_alpha(0.55))
                .top_offset(-13.)
                .bottom_offset(0.4)
                .right_offset(-4.)
                .left_offset(-4.)
                .blur_radius(2.)
                .spread(1.5)
            )
        )
    ))
    .style(|s| {
        s.size_full()
            .items_center()
            .justify_center()
            .selectable(false)
            .col_gap(20.)
    })
}

fn tab_side_item(tab: TabContent, act_tab: RwSignal<Option<usize>>) -> impl IntoView {
    text(format!("{} {}", tab.name, tab.idx)).style(move |s| {
        s.items_center()
            .apply(list_item_class().apply_class(NewListItemClass))
            .justify_center()
            .width_full()
            .height(36.px())
            // .background(BG)
            // .color(TEXT_MUTED)
            // .border(0.5)
            // .selectable(false)
            // .hover(|s| s.border_color(BG_LIGHT).background(BG_LIGHT))
            .border_radius(7.)
            .apply_if(act_tab.get().is_some_and(|a| a == tab.idx), |s| {
                s
                    .apply_selectors(&[StyleSelector::Selected])
                    // .color(TEXT)
                    // .background(BG)
                    // .border(0.)
                    // // .border_color(BG)
                    // .border_top(0.5)
                    // .border_top_color(BG_LIGHT)
                    // .hover(|s| s
                    //     .border(1.)
                    //     .border_color(AQUAMARINE)
                    // )
            })
    })
}

style_class!(pub NewButtonClass);
style_class!(pub NewListItemClass);
// style_class!(pub NewHoverClass);

// fn hover_class() -> Style {
//     Style::new()
//         // .apply_selectors(&[StyleSelector::Selected])
//         // .apply(over)
//         // .apply_class(class)
//         // .apply_custom(custom_style)
//         // .apply_overriding_styles(overrides)
//         .class(NewHoverClass, |s| s
//             .hover(|s| s
//                 .background(BG_LIGHT)
//                 .color(TEXT)
//                 .border_top_color(HIGHLIGHT)
//                 .box_shadow_color(BG_DARK)
//                 .box_shadow_top_offset(-15.)
//                 .box_shadow_bottom_offset(3.)
//                 .box_shadow_right_offset(-6.)
//                 .box_shadow_left_offset(-6.)
//                 .box_shadow_spread(5.)
//                 .box_shadow_blur(4.)
//                 .apply_box_shadow(BoxShadow::new()
//                     .color(BORDER.multiply_alpha(0.8))
//                     .top_offset(-13.)
//                     .bottom_offset(0.4)
//                     .right_offset(-4.)
//                     .left_offset(-4.)
//                     .blur_radius(2.)
//                     .spread(2.)
//                 )
//             )
//         )
// }


fn button_class() -> Style {
    Style::new()
        .class(NewButtonClass, |s| s
            .selectable(false)
            // .transition_background(Transition::linear(30.millis()))
            .background(BG_LIGHT)
            .color(TEXT_MUTED)
            .border(1.)
            .border_color(BG_LIGHT)
            .border_radius(5.)
            .padding_vert(6.px())
            .box_shadow_color(BG_DARK)
            .box_shadow_top_offset(-15.)
            .box_shadow_bottom_offset(3.)
            .box_shadow_right_offset(-6.)
            .box_shadow_left_offset(-6.)
            .box_shadow_spread(4.)
            .box_shadow_blur(4.)
            .apply_box_shadow(BoxShadow::new()
                .color(BORDER.multiply_alpha(0.55))
                .top_offset(-13.)
                .bottom_offset(0.4)
                .right_offset(-4.)
                .left_offset(-4.)
                .blur_radius(2.)
                .spread(1.)
            )
            .hover(|s| s
                .background(BG_LIGHT)
                .color(TEXT)
                .border_top_color(HIGHLIGHT)
                .box_shadow_color(BG_DARK)
                .box_shadow_top_offset(-15.)
                .box_shadow_bottom_offset(3.)
                .box_shadow_right_offset(-6.)
                .box_shadow_left_offset(-6.)
                .box_shadow_spread(5.)
                .box_shadow_blur(4.)
                .apply_box_shadow(BoxShadow::new()
                    .color(BORDER.multiply_alpha(0.8))
                    .top_offset(-13.)
                    .bottom_offset(0.4)
                    .right_offset(-4.)
                    .left_offset(-4.)
                    .blur_radius(2.)
                    .spread(2.)
                )
            )
            .active(|s| s
                .background(BG)
                .color(TEXT)
                .border(0.5)
                .border_color(BG)
                .border_top_color(HIGHLIGHT)
                .box_shadow_color(BG_DARK)
                .box_shadow_top_offset(-15.)
                .box_shadow_bottom_offset(3.)
                .box_shadow_right_offset(-6.)
                .box_shadow_left_offset(-6.)
                .box_shadow_spread(5.)
                .box_shadow_blur(4.)
                .apply_box_shadow(BoxShadow::new()
                    .color(BORDER.multiply_alpha(0.55))
                    .top_offset(-13.)
                    .bottom_offset(0.4)
                    .right_offset(-4.)
                    .left_offset(-4.)
                    .blur_radius(2.)
                    .spread(1.)
                )
            )
            .focus(|s| s
                .selectable(false)
                // .transition_background(Transition::linear(30.millis()))
                .background(BG_LIGHT)
                .color(TEXT_MUTED)
                .border(1.)
                .border_color(BG_LIGHT)
                .border_top(0.5)
                .border_top_color(HIGHLIGHT)
                .border_radius(5.)
                .padding_vert(6.px())
                .box_shadow_color(BG_DARK)
                .box_shadow_top_offset(-15.)
                .box_shadow_bottom_offset(3.)
                .box_shadow_right_offset(-6.)
                .box_shadow_left_offset(-6.)
                .box_shadow_spread(4.)
                .box_shadow_blur(4.)
                .apply_box_shadow(BoxShadow::new()
                    .color(BORDER.multiply_alpha(0.55))
                    .top_offset(-13.)
                    .bottom_offset(0.4)
                    .right_offset(-4.)
                    .left_offset(-4.)
                    .blur_radius(2.)
                    .spread(1.)
                )
                .hover(|s| s
                    .background(BG_LIGHT)
                    .color(TEXT)
                    .border_top_color(HIGHLIGHT)
                    .box_shadow_color(BG_DARK)
                    .box_shadow_top_offset(-15.)
                    .box_shadow_bottom_offset(3.)
                    .box_shadow_right_offset(-6.)
                    .box_shadow_left_offset(-6.)
                    .box_shadow_spread(5.)
                    .box_shadow_blur(4.)
                    .apply_box_shadow(BoxShadow::new()
                        .color(BORDER.multiply_alpha(0.8))
                        .top_offset(-13.)
                        .bottom_offset(0.4)
                        .right_offset(-4.)
                        .left_offset(-4.)
                        .blur_radius(2.)
                        .spread(2.)
                    )
                )
            )
            .focus_visible(|s| s
                .background(YELLOW)
                // .color(TEXT)
                // .border(0.5)
                // .border_color(BORDER)
                // .box_shadow_spread(-0.5)
                // .box_shadow_blur(2.)
                // .box_shadow_v_offset(1.)
                // .box_shadow_color(TEXT)
            )
            .selected(|s| s
                .background(LIGHT_CYAN)
                // .background(BG)
                // .color(TEXT_MUTED)
                // .border(0.5)
                // .border_color(BG_DARK)
                // .border_top_color(BG)
                // .box_shadow_color(BORDER)
                // .box_shadow_top_offset(-15.)
                // .box_shadow_bottom_offset(3.)
                // .box_shadow_right_offset(-6.)
                // .box_shadow_left_offset(-6.)
                // .box_shadow_spread(4.)
                // .box_shadow_blur(4.)
                // .apply_box_shadow(BoxShadow::new()
                //     .color(BORDER.multiply_alpha(0.6))
                //     .top_offset(-13.)
                //     .bottom_offset(0.4)
                //     .right_offset(-4.)
                //     .left_offset(-4.)
                //     .blur_radius(2.)
                //     .spread(1.)
                // )
            )
            .disabled(|s| s
                .background(BG)
                .color(TEXT_MUTED)
                .border(0.5)
                .border_color(BORDER)
                // .border_top_color(BG_LIGHT)
                // .border_bottom_color(BG_DARK)
                .box_shadow_spread(-0.5)
                .box_shadow_blur(2.)
                .box_shadow_v_offset(1.)
                .box_shadow_color(TEXT)
            )
    )
}

fn list_item_class() -> Style {
    Style::new()
        .class(NewListItemClass, |s| s
            .selectable(false)
            // .transition_background(Transition::linear(30.millis()))
            .background(BG_LIGHT)
            .color(TEXT_MUTED)
            .border(1.)
            .border_color(BG_LIGHT)
            .border_radius(5.)
            .padding_vert(6.px())
            .box_shadow_color(BG_DARK)
            .box_shadow_top_offset(-15.)
            .box_shadow_bottom_offset(3.)
            .box_shadow_right_offset(-6.)
            .box_shadow_left_offset(-6.)
            .box_shadow_spread(4.)
            .box_shadow_blur(4.)
            .apply_box_shadow(BoxShadow::new()
                .color(BORDER.multiply_alpha(0.55))
                .top_offset(-13.)
                .bottom_offset(0.4)
                .right_offset(-4.)
                .left_offset(-4.)
                .blur_radius(2.)
                .spread(1.)
            )
            .hover(|s| s
                .background(BG_LIGHT)
                .color(TEXT)
                .border_top_color(HIGHLIGHT)
                .box_shadow_color(BG_DARK)
                .box_shadow_top_offset(-15.)
                .box_shadow_bottom_offset(3.)
                .box_shadow_right_offset(-6.)
                .box_shadow_left_offset(-6.)
                .box_shadow_spread(5.)
                .box_shadow_blur(4.)
                .apply_box_shadow(BoxShadow::new()
                    .color(BORDER.multiply_alpha(0.8))
                    .top_offset(-13.)
                    .bottom_offset(0.4)
                    .right_offset(-4.)
                    .left_offset(-4.)
                    .blur_radius(2.)
                    .spread(2.)
                )
            )
            .active(|s| s
                .background(BG)
                .color(TEXT)
                .border(0.5)
                .inset(0.5)
                .border_color(BG)
                .border_top_color(HIGHLIGHT)
                .box_shadow_color(BG_DARK)
                .box_shadow_top_offset(-15.)
                .box_shadow_bottom_offset(3.)
                .box_shadow_right_offset(-6.)
                .box_shadow_left_offset(-6.)
                .box_shadow_spread(5.)
                .box_shadow_blur(4.)
                .apply_box_shadow(BoxShadow::new()
                    .color(BORDER.multiply_alpha(0.55))
                    .top_offset(-13.)
                    .bottom_offset(0.4)
                    .right_offset(-4.)
                    .left_offset(-4.)
                    .blur_radius(2.)
                    .spread(1.)
                )
            )
            .focus(|s| s
                .selectable(false)
                // .transition_background(Transition::linear(30.millis()))
                .background(BG_LIGHT)
                .color(TEXT_MUTED)
                .border(1.)
                .border_color(BG_LIGHT)
                .border_top(0.5)
                .border_top_color(HIGHLIGHT)
                .border_radius(5.)
                .padding_vert(6.px())
                .box_shadow_color(BG_DARK)
                .box_shadow_top_offset(-15.)
                .box_shadow_bottom_offset(3.)
                .box_shadow_right_offset(-6.)
                .box_shadow_left_offset(-6.)
                .box_shadow_spread(4.)
                .box_shadow_blur(4.)
                .apply_box_shadow(BoxShadow::new()
                    .color(BORDER.multiply_alpha(0.55))
                    .top_offset(-13.)
                    .bottom_offset(0.4)
                    .right_offset(-4.)
                    .left_offset(-4.)
                    .blur_radius(2.)
                    .spread(1.)
                )
                .hover(|s| s
                    .background(BG_LIGHT)
                    .color(TEXT)
                    .border_top_color(HIGHLIGHT)
                    .box_shadow_color(BG_DARK)
                    .box_shadow_top_offset(-15.)
                    .box_shadow_bottom_offset(3.)
                    .box_shadow_right_offset(-6.)
                    .box_shadow_left_offset(-6.)
                    .box_shadow_spread(5.)
                    .box_shadow_blur(4.)
                    .apply_box_shadow(BoxShadow::new()
                        .color(BORDER.multiply_alpha(0.8))
                        .top_offset(-13.)
                        .bottom_offset(0.4)
                        .right_offset(-4.)
                        .left_offset(-4.)
                        .blur_radius(2.)
                        .spread(2.)
                    )
                )
            )
            .focus_visible(|s| s
                .background(YELLOW)
                // .color(TEXT)
                // .border(0.5)
                // .border_color(BORDER)
                // .box_shadow_spread(-0.5)
                // .box_shadow_blur(2.)
                // .box_shadow_v_offset(1.)
                // .box_shadow_color(TEXT)
            )
            .selected(|s| s
                .background(BG)
                .color(TEXT)
                .border(0.5)
                .border_color(BG)
                .border_top_color(HIGHLIGHT)
                .box_shadow_color(BG_DARK)
                .box_shadow_top_offset(-15.)
                .box_shadow_bottom_offset(3.)
                .box_shadow_right_offset(-6.)
                .box_shadow_left_offset(-6.)
                .box_shadow_spread(4.)
                .box_shadow_blur(4.)
                .apply_box_shadow(BoxShadow::new()
                    .color(BORDER.multiply_alpha(0.55))
                    .top_offset(-13.)
                    .bottom_offset(0.4)
                    .right_offset(-4.)
                    .left_offset(-4.)
                    .blur_radius(2.)
                    .spread(1.)
                )
                .hover(|s| s
                    .background(BG)
                    .color(TEXT)
                    .border(0.5)
                    .border_color(BG)
                    .border_top_color(HIGHLIGHT)
                    .box_shadow_color(BG_DARK)
                    .box_shadow_top_offset(-15.)
                    .box_shadow_bottom_offset(3.)
                    .box_shadow_right_offset(-6.)
                    .box_shadow_left_offset(-6.)
                    .box_shadow_spread(5.)
                    .box_shadow_blur(4.)
                    .apply_box_shadow(BoxShadow::new()
                        .color(BORDER.multiply_alpha(0.8))
                        .top_offset(-13.)
                        .bottom_offset(0.4)
                        .right_offset(-4.)
                        .left_offset(-4.)
                        .blur_radius(2.)
                        .spread(2.)
                    )
                )
                // .background(LIGHT_CYAN)
                // .color(TEXT)
                // .background(BG)
                // .border(0.)
                // // .border_color(BG)
                // .border_top(0.5)
                // .border_top_color(BG_LIGHT)
                // .hover(|s| s
                //     .border(1.)
                //     .border_color(AQUAMARINE)
                // )
            )
            .disabled(|s| s
                .background(BG)
                .color(TEXT_MUTED)
                .border(0.5)
                .border_color(BORDER)
                // .border_top_color(BG_LIGHT)
                // .border_bottom_color(BG_DARK)
                .box_shadow_spread(-0.5)
                .box_shadow_blur(2.)
                .box_shadow_v_offset(1.)
                .box_shadow_color(TEXT)
            )
    )
}

const BG_DARK: Color = hsl([60., 3., 90.]);
const BG: Color = hsl([60., 3., 95.]);
const BG_LIGHT: Color = hsl([60., 3., 100.]);

const BORDER: Color = hsl([60., 3., 60.]);
const HIGHLIGHT: Color = hsl([60., 3., 100.]);

const TEXT: Color = hsl([60., 3., 5.]);
const TEXT_MUTED: Color = hsl([60., 3., 30.]);

const fn hsl([h, s, l]: [f32; 3]) -> Color {
    let sat = s * 0.01;
    let light = l * 0.01;
    let a = sat * light.min(1.0 - light);

    let hh = transform(0., [h, s, l]);
    let ss = transform(8., [h, s, l]);
    let ll = transform(4., [h, s, l]);
    let rgb = Color::new([hh, ss, ll, 1.]);
    rgb
}

const fn transform(n: f32, [h, s, l]: [f32; 3]) -> f32 {
    let sat = s * 0.01;
    let light = l * 0.01;
    let a = sat * light.min(1.0 - light);

    let x = n + h * (1.0 / 30.0);
    let k = x - 12.0 * (x * (1.0 / 12.0)).floor();
    light - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
}
