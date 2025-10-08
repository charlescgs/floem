#![allow(unused)]
use floem::peniko::color::ColorSpace;
use floem::peniko::color::Hsl;
use floem::prelude::palette::css;
use floem::prelude::palette::css::BLACK;
use floem::prelude::palette::css::DARK_GRAY;
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
            move |tab| tab_side_view(tab.name).on_click_stop(move |_| {
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
    )).style(|s| s.size_full());
    
    v_stack((
        h_stack((
            button("add tab").action(move || tab_action.update(|a| {*a = Action::Add;}))
                .style(|s| s
                    .background(hsl([0., 0., 0.]))
                ),
            button("remove tab").action(move || tab_action.update(|a| {*a = Action::Remove;})),
        )).style(|s| s
            .height(40.px())
            .width_full()
            .border_bottom(1.)
            .border_color(css::BLACK)
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
        ),
    )).style(|s| s
        .size_full()
        .border(1.)
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

fn tab_side_view(name: String) -> impl IntoView {
    text(name).style(|s| s
        .items_center()
        .justify_center()
        .width_full()
        .height(36.px())
        .border(0.5)
        .border_radius(5.)
        .hover(|s| s.apply_class(ListItemClass))
    )
}

fn hsl(c: [f32; 3]) -> Color {
    let hsl = Hsl::to_linear_srgb([c[0], c[1], c[2]]);
    let rgb = Color::new([hsl[0], hsl[1], hsl[2], 1.]);
    rgb
}