use gtk::gdk;

pub fn load() {
    let provider = gtk::CssProvider::new();

    provider.load_from_data(
        r#"
.ferru-wifi-popover {
  border-radius: 24px;
}

.ferru-wifi-header-icon {
  /* Visually matches switch height better */
  margin-right: 2px;
}

.ferru-wifi-title {
  font-weight: 700;
  font-size: 16px;
}

/* Rounded backdrop card */
.ferru-wifi-card {
  border-radius: 18px;
  padding: 10px;
  background-color: alpha(@theme_fg_color, 0.04);
}

/* Kill the square behind by forcing internal containers transparent */
.ferru-wifi-card scrolledwindow,
.ferru-wifi-card viewport,
.ferru-wifi-card list,
.ferru-wifi-card .view {
  background: transparent;
}

.ferru-wifi-list row {
  background: transparent;
  padding: 0;
  margin: 0;
  border: 0;
  box-shadow: none;
  outline: none;
}

/* Pills */
.ferru-wifi-pill {
  border-radius: 14px;
  padding: 10px 12px;
  background-color: alpha(@theme_fg_color, 0.07);
}

.ferru-wifi-row:hover .ferru-wifi-pill {
  background-color: alpha(@theme_fg_color, 0.10);
}

.ferru-wifi-row:active .ferru-wifi-pill {
  background-color: alpha(@theme_fg_color, 0.13);
}

.ferru-wifi-active .ferru-wifi-pill {
  background-color: alpha(@accent_bg_color, 0.22);
}

/* Footer button */
.ferru-wifi-footer-btn {
  border-radius: 18px;
  background-color: alpha(@theme_fg_color, 0.04);
  padding: 0;
}

.ferru-wifi-footer-btn:hover {
  background-color: alpha(@theme_fg_color, 0.06);
}

.ferru-wifi-sep {
  margin-top: 8px;
  margin-bottom: 8px;
  opacity: 0.25;
}

/* Undershoot gradients */
.ferru-wifi-undershoot-top {
  background-image: linear-gradient(
    to bottom,
    alpha(@window_bg_color, 0.55),
    alpha(@window_bg_color, 0.0)
  );
}

.ferru-wifi-undershoot-bottom {
  background-image: linear-gradient(
    to top,
    alpha(@window_bg_color, 0.55),
    alpha(@window_bg_color, 0.0)
  );
}
"#,
    );

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("No display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

