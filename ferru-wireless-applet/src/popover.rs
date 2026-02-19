use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use crate::actions;
use crate::wireless;

pub struct WirelessPopover {
    popover: gtk::Popover,

    list: gtk::ListBox,
    stack: gtk::Stack,

    wifi_switch: gtk::Switch,

    state_icon: gtk::Image,
    state_label: gtk::Label,
    spinner: gtk::Spinner,

    footer_btn: gtk::Button,

    // guard to prevent switch notify recursion
    inhibit_switch: Rc<Cell<bool>>,
}

impl WirelessPopover {
    pub fn new(anchor: &impl IsA<gtk::Widget>) -> Self {
        let popover = gtk::Popover::new();
        popover.set_has_arrow(false);
        popover.set_autohide(true);
        popover.set_parent(anchor.as_ref());
        popover.add_css_class("ferru-wifi-popover");

        let (pw, ph) = scaled_popover_size();
        popover.set_size_request(pw, ph);

        // Root container
        let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
        root.set_hexpand(true);
        root.set_vexpand(true);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);

        // Header: [icon] Wi-Fi ...... [switch]
        let header = gtk::CenterBox::new();
        header.add_css_class("ferru-wifi-header");

        let left = gtk::Box::new(gtk::Orientation::Horizontal, 10);

        let header_icon = gtk::Image::from_icon_name("network-wireless-symbolic");
        header_icon.add_css_class("ferru-wifi-header-icon");
        header_icon.set_pixel_size(22);
        header_icon.set_valign(gtk::Align::Center);

        let title = gtk::Label::new(Some("Wi-Fi"));
        title.add_css_class("ferru-wifi-title");
        title.set_halign(gtk::Align::Start);

        left.append(&header_icon);
        left.append(&title);

        let wifi_switch = gtk::Switch::new();
        wifi_switch.set_valign(gtk::Align::Center);

        header.set_start_widget(Some(&left));
        header.set_end_widget(Some(&wifi_switch));

        // Content card (clips children so no inner “square” bleeds through)
        let content_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content_card.add_css_class("ferru-wifi-card");
        content_card.set_hexpand(true);
        content_card.set_vexpand(true);
        content_card.set_overflow(gtk::Overflow::Hidden);

        // Stack: "state" vs "list"
        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        stack.set_transition_duration(120);

        // --- State page (centered)
        let state_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        state_box.set_hexpand(true);
        state_box.set_vexpand(true);
        state_box.set_halign(gtk::Align::Center);
        state_box.set_valign(gtk::Align::Center);
        state_box.add_css_class("ferru-wifi-state");

        let spinner = gtk::Spinner::new();
        spinner.set_spinning(false);

        let state_icon = gtk::Image::from_icon_name("network-wireless-signal-none-symbolic");
        state_icon.set_pixel_size(72);
        state_icon.set_opacity(0.25);

        let state_label = gtk::Label::new(Some("No Wi-Fi networks found"));
        state_label.set_wrap(true);
        state_label.set_halign(gtk::Align::Center);
        state_label.add_css_class("ferru-wifi-state-label");

        state_box.append(&spinner);
        state_box.append(&state_icon);
        state_box.append(&state_label);

        stack.add_named(&state_box, Some("state"));

        // --- List page (scroller + undershoot)
        let overlay = gtk::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);

        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .has_frame(false)
            .build();
        scroller.set_hexpand(true);
        scroller.set_vexpand(true);
        scroller.set_overlay_scrolling(true);

        let list = gtk::ListBox::new();
        list.add_css_class("ferru-wifi-list");
        list.set_selection_mode(gtk::SelectionMode::None);
        list.set_activate_on_single_click(true);
        list.set_show_separators(false);

        scroller.set_child(Some(&list));
        overlay.set_child(Some(&scroller));

        // Undershoot revealers
        let fade_top = gtk::Revealer::new();
        fade_top.add_css_class("ferru-wifi-undershoot");
        fade_top.set_valign(gtk::Align::Start);
        fade_top.set_halign(gtk::Align::Fill);
        fade_top.set_transition_type(gtk::RevealerTransitionType::Crossfade);
        fade_top.set_transition_duration(110);
        fade_top.set_reveal_child(false);
        fade_top.set_can_target(false);

        let fade_top_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        fade_top_box.add_css_class("ferru-wifi-undershoot-top");
        fade_top_box.set_height_request(18);
        fade_top.set_child(Some(&fade_top_box));

        let fade_bottom = gtk::Revealer::new();
        fade_bottom.add_css_class("ferru-wifi-undershoot");
        fade_bottom.set_valign(gtk::Align::End);
        fade_bottom.set_halign(gtk::Align::Fill);
        fade_bottom.set_transition_type(gtk::RevealerTransitionType::Crossfade);
        fade_bottom.set_transition_duration(110);
        fade_bottom.set_reveal_child(false);
        fade_bottom.set_can_target(false);

        let fade_bottom_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        fade_bottom_box.add_css_class("ferru-wifi-undershoot-bottom");
        fade_bottom_box.set_height_request(18);
        fade_bottom.set_child(Some(&fade_bottom_box));

        overlay.add_overlay(&fade_top);
        overlay.add_overlay(&fade_bottom);

        bind_undershoot(&scroller, &fade_top, &fade_bottom);

        stack.add_named(&overlay, Some("list"));

        content_card.append(&stack);

        // Separator + footer pill
        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        sep.add_css_class("ferru-wifi-sep");

        let footer_btn = gtk::Button::new();
        footer_btn.add_css_class("ferru-wifi-footer-btn");
        footer_btn.set_halign(gtk::Align::Fill);

        let footer_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        footer_row.set_halign(gtk::Align::Fill);
        footer_row.set_margin_start(14);
        footer_row.set_margin_end(14);
        footer_row.set_margin_top(12);
        footer_row.set_margin_bottom(12);

        let footer_lbl = gtk::Label::new(Some("All Networks"));
        footer_lbl.set_xalign(0.0);
        footer_lbl.set_hexpand(true);
        footer_row.append(&footer_lbl);

        footer_btn.set_child(Some(&footer_row));

        // Compose
        root.append(&header);
        root.append(&content_card);
        root.append(&sep);
        root.append(&footer_btn);
        popover.set_child(Some(&root));

        let this = Self {
            popover,
            list,
            stack,
            wifi_switch,
            state_icon,
            state_label,
            spinner,
            footer_btn,
            inhibit_switch: Rc::new(Cell::new(false)),
        };

        this.wire();
        this.show_idle_empty();
        this
    }

    pub fn popover(&self) -> &gtk::Popover {
        &self.popover
    }

    pub fn refresh(&self) {
        self.set_loading("Searching for Wi-Fi networks…");

        let sw = self.wifi_switch.clone();
        let inhibit = self.inhibit_switch.clone();

        let list = self.list.clone();
        let stack = self.stack.clone();

        let state_icon = self.state_icon.clone();
        let state_label = self.state_label.clone();
        let spinner = self.spinner.clone();

        wireless::wifi_enabled_async(move |res| {
            let enabled = res.unwrap_or(true);

            // Update switch (avoid recursion)
            inhibit.set(true);
            sw.set_active(enabled);
            inhibit.set(false);

            if !enabled {
                clear_listbox(&list);
                spinner.set_spinning(false);
                stack.set_visible_child_name("state");
                state_icon.set_icon_name(Some("network-wireless-disabled-symbolic"));
                state_icon.set_pixel_size(72);
                state_icon.set_opacity(0.25);
                state_label.set_text("Wi-Fi is disabled");
                return;
            }

            spinner.set_spinning(true);
            wireless::scan_access_points_async(move |res2| {
                spinner.set_spinning(false);
                match res2 {
                    Ok(aps) if !aps.is_empty() => {
                        populate_list(&list, &aps);
                        stack.set_visible_child_name("list");
                    }
                    Ok(_) => {
                        clear_listbox(&list);
                        stack.set_visible_child_name("state");
                        state_icon.set_icon_name(Some("network-wireless-signal-none-symbolic"));
                        state_icon.set_pixel_size(72);
                        state_icon.set_opacity(0.25);
                        state_label.set_text("No Wi-Fi networks found");
                    }
                    Err(_) => {
                        clear_listbox(&list);
                        stack.set_visible_child_name("state");
                        state_icon.set_icon_name(Some("dialog-warning-symbolic"));
                        state_icon.set_pixel_size(48);
                        state_icon.set_opacity(0.25);
                        state_label.set_text("Could not scan Wi-Fi networks");
                    }
                }
            });
        });
    }

    fn set_loading(&self, msg: &str) {
        clear_listbox(&self.list);
        self.stack.set_visible_child_name("state");
        self.spinner.set_spinning(true);
        self.state_icon.set_icon_name(Some("network-wireless-signal-none-symbolic"));
        self.state_icon.set_pixel_size(48);
        self.state_icon.set_opacity(0.0);
        self.state_label.set_text(msg);
    }

    fn show_idle_empty(&self) {
        self.stack.set_visible_child_name("state");
        self.spinner.set_spinning(false);
        self.state_icon.set_icon_name(Some("network-wireless-signal-none-symbolic"));
        self.state_icon.set_pixel_size(72);
        self.state_icon.set_opacity(0.25);
        self.state_label.set_text("No Wi-Fi networks found");
    }

    fn wire(&self) {
        // “All Networks”
        let pop = self.popover.clone();
        self.footer_btn.connect_clicked(move |_| {
            let _ = actions::open_wifi_settings();
            pop.popdown();
        });

        // Switch toggle
        let inhibit = self.inhibit_switch.clone();
        let sw = self.wifi_switch.clone();

        let list = self.list.clone();
        let stack = self.stack.clone();
        let state_icon = self.state_icon.clone();
        let state_label = self.state_label.clone();
        let spinner = self.spinner.clone();

        self.wifi_switch
            .connect_notify_local(Some("active"), move |s, _| {
                if inhibit.get() {
                    return;
                }

                let enable = s.is_active();

                // Immediate UI feedback
                clear_listbox(&list);
                stack.set_visible_child_name("state");
                spinner.set_spinning(true);
                state_icon.set_opacity(0.0);
                state_label.set_text(if enable { "Enabling Wi-Fi…" } else { "Disabling Wi-Fi…" });

                let sw_u = sw.clone();
                let inhibit_u = inhibit.clone();

                let list_u = list.clone();
                let stack_u = stack.clone();
                let state_icon_u = state_icon.clone();
                let state_label_u = state_label.clone();
                let spinner_u = spinner.clone();

                wireless::set_wifi_enabled_async(enable, move |res| {
                    spinner_u.set_spinning(false);

                    if res.is_err() {
                        inhibit_u.set(true);
                        sw_u.set_active(!enable);
                        inhibit_u.set(false);

                        stack_u.set_visible_child_name("state");
                        state_icon_u.set_icon_name(Some("dialog-warning-symbolic"));
                        state_icon_u.set_pixel_size(48);
                        state_icon_u.set_opacity(0.25);
                        state_label_u.set_text("Could not change Wi-Fi state");
                        return;
                    }

                    if !enable {
                        clear_listbox(&list_u);
                        stack_u.set_visible_child_name("state");
                        state_icon_u.set_icon_name(Some("network-wireless-disabled-symbolic"));
                        state_icon_u.set_pixel_size(72);
                        state_icon_u.set_opacity(0.25);
                        state_label_u.set_text("Wi-Fi is disabled");
                        return;
                    }

                    // Give NM a short beat, then scan (GNOME-like cadence)
                    spinner_u.set_spinning(true);
                    glib::timeout_add_local_once(Duration::from_millis(180), move || {
                        let list_r = list_u.clone();
                        let stack_r = stack_u.clone();
                        let state_icon_r = state_icon_u.clone();
                        let state_label_r = state_label_u.clone();
                        let spinner_r = spinner_u.clone();

                        wireless::scan_access_points_async(move |res2| {
                            spinner_r.set_spinning(false);
                            match res2 {
                                Ok(aps) if !aps.is_empty() => {
                                    populate_list(&list_r, &aps);
                                    stack_r.set_visible_child_name("list");
                                }
                                Ok(_) => {
                                    clear_listbox(&list_r);
                                    stack_r.set_visible_child_name("state");
                                    state_icon_r.set_icon_name(Some(
                                        "network-wireless-signal-none-symbolic",
                                    ));
                                    state_icon_r.set_pixel_size(72);
                                    state_icon_r.set_opacity(0.25);
                                    state_label_r.set_text("No Wi-Fi networks found");
                                }
                                Err(_) => {
                                    clear_listbox(&list_r);
                                    stack_r.set_visible_child_name("state");
                                    state_icon_r.set_icon_name(Some("dialog-warning-symbolic"));
                                    state_icon_r.set_pixel_size(48);
                                    state_icon_r.set_opacity(0.25);
                                    state_label_r.set_text("Could not scan Wi-Fi networks");
                                }
                            }
                        });
                    });
                });
            });

        // Row activation behavior
        let pop = self.popover.clone();
        self.list.connect_row_activated(move |_l, row| {
            let ssid = row.widget_name().to_string();
            if ssid.trim().is_empty() {
                return;
            }

            if row.has_css_class("ferru-wifi-active") {
                pop.popdown();
                return;
            }

            let _ = actions::open_wifi_settings();
            pop.popdown();
        });
    }
}

fn bind_undershoot(scroller: &gtk::ScrolledWindow, top: &gtk::Revealer, bottom: &gtk::Revealer) {
    let adj = scroller.vadjustment();

    let top = top.clone();
    let bottom = bottom.clone();

    // GNOME-ish: don’t flicker at the exact ends; use a small epsilon
    let update: Rc<dyn Fn(&gtk::Adjustment)> = Rc::new(move |a: &gtk::Adjustment| {
        let upper = a.upper();
        let page = a.page_size();
        let val = a.value();

        let scrollable = upper > (page + 1.0);
        let eps = 0.75;

        let at_top = val <= eps;
        let at_bottom = (val + page) >= (upper - eps);

        top.set_reveal_child(scrollable && !at_top);
        bottom.set_reveal_child(scrollable && !at_bottom);
    });

    {
        let u = update.clone();
        adj.connect_value_changed(move |a| u(a));
    }
    {
        let u = update.clone();
        adj.connect_changed(move |a| u(a));
    }

    update(&adj);
}

fn scaled_popover_size() -> (i32, i32) {
    let fallback = (360, 520);

    let Some(display) = gdk::Display::default() else { return fallback; };

    let monitors = display.monitors();
    let Some(obj) = monitors.item(0) else { return fallback; };
    let Ok(mon) = obj.downcast::<gdk::Monitor>() else { return fallback; };

    let geo = mon.geometry();
    let w = (geo.width() as f32 * 0.20).round() as i32;
    let h = (geo.height() as f32 * 0.35).round() as i32;

    (w.clamp(280, 520), h.clamp(360, 760))
}

fn clear_listbox(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn ap_row(ap: &wireless::AccessPoint) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);
    row.set_selectable(false);
    row.set_widget_name(&ap.ssid);

    row.add_css_class("ferru-wifi-row");
    if ap.active {
        row.add_css_class("ferru-wifi-active");
    }

    // Pill (the ONLY painted surface per row)
    let pill = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    pill.add_css_class("ferru-wifi-pill");
    pill.set_margin_start(8);
    pill.set_margin_end(8);
    pill.set_margin_top(6);
    pill.set_margin_bottom(6);
    pill.set_hexpand(true);

    let icon = gtk::Image::from_icon_name(wireless::signal_icon(ap.signal));
    icon.set_pixel_size(18);

    let label = gtk::Label::new(Some(&ap.ssid));
    label.set_xalign(0.0);
    label.set_hexpand(true);

    let trailing = gtk::Box::new(gtk::Orientation::Horizontal, 10);

    if ap.secure {
        let lock = gtk::Image::from_icon_name("changes-prevent-symbolic");
        lock.set_pixel_size(16);
        trailing.append(&lock);
    }

    if ap.active {
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        check.set_pixel_size(16);
        trailing.append(&check);
    }

    pill.append(&icon);
    pill.append(&label);
    pill.append(&trailing);

    row.set_child(Some(&pill));
    row
}

fn populate_list(list: &gtk::ListBox, aps: &[wireless::AccessPoint]) {
    clear_listbox(list);
    for ap in aps {
        list.append(&ap_row(ap));
    }
}

