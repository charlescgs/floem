use floem::prelude::*;

struct TabContent {
    idx: usize,
    id: u64,
    color: Color
}

pub fn tab_view() -> impl IntoView {
    let tabs = RwSignal::new(vec!());
    let active_tab = RwSignal::new(None);

    let tabs_view = stack(
        dyn_stack(
            move || tabs.get(),
            move |tab| tab.idx,
            move |tab| tab
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
        .debug_name("rooms scroll")
        .style(|s| s
            .size_full()
            .padding(5.)
            .padding_right(7.)
        ).scroll_style(|s| s
            .handle_thickness(6.)
            .shrink_to_fit()
        ),
    );


    let tabs_content_view = stack((
        tab(
            move || active_tab.get(),
            move || tabs.get(),
            |tab| tab.idx,
            move |tab| tab
        ),
    ));

    todo!()
}
