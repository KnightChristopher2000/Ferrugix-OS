use adw::prelude::*;

use ferru_wireless_applet::{popover::WirelessPopover, style};

fn main() {
    let app = adw::Application::builder()
        .application_id("org.ferru.WirelessApplet")
        .build();

    app.connect_activate(|app| {
        style::load();

        // Stub host window (until Ferru Panel exists)
        let win = adw::ApplicationWindow::builder()
            .application(app)
            .title("Ferru Wireless Applet")
            .default_width(88)
            .default_height(56)
            .build();

        win.set_decorated(false);
        win.set_resizable(false);
        win.add_css_class("ferru-host-window");

        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.add_css_class("ferru-host-root");

        let btn = gtk::Button::new();
        btn.add_css_class("ferru-host-btn");

        let icon = gtk::Image::from_icon_name("network-wireless-symbolic");
        icon.set_pixel_size(18);
        btn.set_child(Some(&icon));

        let pop = WirelessPopover::new(&btn);

        btn.connect_clicked(move |_| {
            pop.refresh();
            pop.popover().popup();
        });

        root.append(&btn);
        win.set_content(Some(&root));
        win.present();
    });

    app.run();
}

