#![allow(unused)]
use floem::peniko::color::ColorSpace;
use floem::peniko::color::Hsl;
use floem::prelude::palette::css;
use floem::prelude::palette::css::BLACK;
use floem::prelude::palette::css::DARK_GRAY;
use floem::prelude::palette::css::WHITE;
use floem::prelude::*;
use floem::reactive::create_effect;
use floem::style;

use crate::form;
use crate::form::form_item;


#[derive(Clone)]
struct TabContent {
    idx: usize,
    name: String
}

impl TabContent {
    fn new(tabs_count: usize) -> Self {
        Self {
            idx: tabs_count,
            name: format!("Tab with index {tabs_count}")
        }
    }
}

#[derive(Clone)]
enum Action {
    Add,
    Remove,
    None
}


pub fn tab_view() -> impl IntoView {
    let tabs = RwSignal::new(vec!());
    let active_tab = RwSignal::new(None::<usize>);
    let tab_action = RwSignal::new(Action::None);

    create_effect(move |_| {
        match tab_action.get() {
            Action::Add => {
                tabs.update(|tabs| {
                    tabs.push(TabContent::new(tabs.len()))
                });
            },
            Action::Remove => {
                tabs.update(|tabs| {
                    tabs.pop();
                });
            },
            Action::None => ()
        }
    });

    let tabs_view = stack((
        dyn_stack(
            move || tabs.get(),
            |tab| tab.idx,
            move |tab| tab_side_item(tab.clone(), active_tab).on_click_stop(move |_| {
                active_tab.update(|a| {*a = Some(tab.idx);});
            })
        ).style(|s| s
            .flex_col()
            .width_full()
            .row_gap(5.)
        )
        .scroll()
        .on_click_stop(move |_| {
            if active_tab.with_untracked(|act| act.is_some()) {
                active_tab.set(None)
            }
        })
        .style(|s| s
            .size_full()
            .padding(5.)
            .padding_right(7.)
        )
        .scroll_style(|s| s
            .handle_thickness(6.)
            .shrink_to_fit()
        ),
    )).style(|s| s
        .width(120.)
        .min_width(120.)
        .height_full()
        .background(BG)
        .border_right(1.)
        .border_color(css::BLACK)
    );


    let tabs_content_view = stack((
        tab(
            move || active_tab.get(),
            move || tabs.get(),
            |tab| tab.idx,
            move |tab| show_tab_content(tab)
        ).style(|s| s.size_full()),
    )).style(|s| s.size_full().background(BG));

    v_stack((
        h_stack((
            button("add tab").action(move || tab_action.update(|a| {*a = Action::Add;}))
                .style(|s| s
                    .background(BG_LIGHT)
                    .border(0.)
                    .border_bottom(0.5)
                    .border_top(0.5)
                    .border_right(0.)
                    .border_left(0.)
                    .border_top_color(WHITE)
                    .border_bottom_color(BG_DARK)
                    .box_shadow_blur(1.)
                    .box_shadow_v_offset(2.)
                    .box_shadow_color(BG_DARK)
                ),
            button("remove tab").action(move || tab_action.update(|a| {*a = Action::Remove;}))
                .style(|s| s
                    .background(BG_LIGHT)
                    .border(0.)
                    .border_bottom(0.5)
                    .border_top(0.5)
                    .border_right(0.)
                    .border_left(0.)
                    .border_top_color(WHITE)
                    .border_bottom_color(BG_DARK)
                    .box_shadow_blur(1.)
                    .box_shadow_v_offset(2.)
                    .box_shadow_color(BG_DARK)
                ),
        )).style(|s| s
            .height(40.px())
            .width_full()
            .border_bottom(1.)
            .border_color(css::BLACK)
            .background(BG)
            .padding(5.)
            .col_gap(5.)
            .items_center()
        ),
        stack((
            tabs_view,
            tabs_content_view
        )).style(|s| s
            .height(300.px())
            .width(400.px())
            .background(BG)
        ),
    )).style(|s| s
        .size_full()
        .border(1.)
        .background(BG_DARK)
        .border_radius(6.)
        .border_color(BLACK)
    )
}

fn show_tab_content(tab: TabContent) -> impl IntoView {
    stack((
        label(move || format!("{} is now active!", tab.name))
            .style(|s| s.font_size(18.).font_bold()),
    )).style(|s| s
        .size_full()
        .items_center()
        .justify_center()
    )
}

fn tab_side_item(tab: TabContent, act_tab: RwSignal<Option<usize>>) -> impl IntoView {
    text(tab.name).style(move |s| s
        .items_center()
        .justify_center()
        .width_full()
        .height(36.px())
        .background(BG_LIGHT)
        .border(0.5)
        .border_radius(5.)
        .hover(|s| s.border_color(BG_DARK).background(BG))
        .apply_if(
            act_tab.get().is_some_and(|a| a == tab.idx),
            |s| s.background(BG_DARK).border_color(BG_LIGHT)
        )
    )
}


const BG_DARK: Color = hsl([0., 0., 90.]);
const BG: Color = hsl([0., 0., 95.]);
const BG_LIGHT: Color = hsl([0., 0., 100.]);

const TEXT: Color = hsl([0., 0., 5.]);
const TEXT_MUTED: Color = hsl([0.0, 0., 30.]);

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