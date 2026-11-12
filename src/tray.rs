//! The capsule and the two menus behind it.
//!
//! One status item, one menu, two halves. `Network ▸` is pingbar's monitor —
//! latency, loss, throughput, a speed test, the target. `Xproxy ▸` is the
//! gateway — whether it is serving, where, how many models it knows, and the
//! four things you would otherwise open a terminal for.
//!
//! The capsule shows one number on purpose: latency is what you glance at. The
//! gateway gets a single glyph — `⏸` when it is not serving — and everything
//! else lives in the menu, where there is room to read it.

use std::cell::RefCell;
use std::net::IpAddr;

use pingbar::Snapshot;
use pingbar::menubar::capsule::Capsule;
use tray_icon::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};
use tray_icon::{TrayIcon, TrayIconBuilder};
use xproxy::Status;

use crate::gateway::{Copy, GatewayHandle};

/// Preset targets, router first: the box you are actually connected to says
/// more about your link than a public resolver does.
const PRESETS: [(&str, &str); 4] = [
    ("Router  ·  192.168.1.1", "192.168.1.1"),
    ("Google DNS  ·  8.8.8.8", "8.8.8.8"),
    ("Cloudflare  ·  1.1.1.1", "1.1.1.1"),
    ("Quad9  ·  9.9.9.9", "9.9.9.9"),
];

fn ip(literal: &str) -> IpAddr {
    literal.parse().expect("preset target is a valid IP literal")
}

pub struct TrayUi {
    tray: TrayIcon,
    capsule: Capsule,

    net_status: MenuItem,
    net_speed: MenuItem,
    net_test: MenuItem,
    targets: [CheckMenuItem; PRESETS.len()],

    xp_status: MenuItem,
    xp_detail: MenuItem,
    xp_toggle: MenuItem,
    xp_url: MenuItem,
    xp_key: MenuItem,
    xp_refresh: MenuItem,

    quit: MenuItem,
    last_title: RefCell<String>,
}

impl TrayUi {
    pub fn build(control: &pingbar::Control, gateway: &GatewayHandle) -> Result<Self, String> {
        let net_status = MenuItem::new("Starting…", false, None);
        let net_speed = MenuItem::new("No data yet", false, None);
        let net_test = MenuItem::new("Speed test", true, None);

        let current = control.config().ping_target;
        let targets = PRESETS.map(|(label, literal)| {
            CheckMenuItem::new(label, true, current == ip(literal), None)
        });
        let target_menu = Submenu::new("Target", true);
        let entries: Vec<&dyn IsMenuItem> = targets.iter().map(|item| item as &dyn IsMenuItem).collect();
        target_menu
            .append_items(&entries)
            .map_err(|error| error.to_string())?;

        let network_menu = Submenu::new("Network", true);
        network_menu
            .append_items(&[
                &net_status,
                &net_speed,
                &PredefinedMenuItem::separator(),
                &net_test,
                &target_menu,
            ])
            .map_err(|error| error.to_string())?;

        let xp_status = MenuItem::new("Starting…", false, None);
        let xp_detail = MenuItem::new("", false, None);
        let xp_toggle = MenuItem::new("Pause", false, None);
        let xp_url = MenuItem::new("Copy Base URL", false, None);
        let xp_key = MenuItem::new("Copy Gateway Key", false, None);
        let xp_refresh = MenuItem::new("Refresh Tokens", false, None);

        let gateway_menu = Submenu::new("Xproxy", true);
        gateway_menu
            .append_items(&[
                &xp_status,
                &xp_detail,
                &PredefinedMenuItem::separator(),
                &xp_toggle,
                &xp_url,
                &xp_key,
                &PredefinedMenuItem::separator(),
                &xp_refresh,
            ])
            .map_err(|error| error.to_string())?;

        let quit = MenuItem::new("Quit Opus", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &network_menu,
            &gateway_menu,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|error| error.to_string())?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Opus — network and gateway")
            .build()
            .map_err(|error| error.to_string())?;
        tray.set_title(Some("…"));

        let capsule = Capsule::attach(&tray);
        capsule.refresh("…");

        Ok(Self {
            tray,
            capsule,
            net_status,
            net_speed,
            net_test,
            targets,
            xp_status,
            xp_detail,
            xp_toggle,
            xp_url,
            xp_key,
            xp_refresh,
            quit,
            last_title: RefCell::new(String::new()),
        })
    }

    /// Paint a monitor snapshot into the capsule and the Network submenu.
    pub fn update_network(&self, snapshot: &Snapshot) {
        let title = capsule_title(snapshot);
        if *self.last_title.borrow() != title {
            self.tray.set_title(Some(title.as_str()));
            *self.last_title.borrow_mut() = title.clone();
        }
        // Unconditional: AppKit drops the fill and the title when the
        // appearance or the highlight changes, and re-asserting is cheap.
        self.capsule.refresh(&title);

        self.net_status.set_text(if let Some(error) = &snapshot.error {
            format!("Error: {error}")
        } else if snapshot.samples == 0 {
            "Starting…".to_owned()
        } else if snapshot.online() {
            let ping = snapshot
                .ping_ms
                .map(|ping| format!("{ping:.0} ms"))
                .unwrap_or_else(|| "—".to_owned());
            format!("{ping}  ·  {:.0}% loss", snapshot.loss_pct)
        } else {
            "RTO — no reply".to_owned()
        });

        self.net_speed.set_text(if snapshot.speed_running {
            "Measuring…".to_owned()
        } else {
            match (snapshot.download_mbps, snapshot.upload_mbps) {
                (Some(down), Some(up)) => format!("↓ {down:.1}  ·  ↑ {up:.1} Mbps"),
                (Some(down), None) => format!("↓ {down:.1} Mbps"),
                (None, Some(up)) => format!("↑ {up:.1} Mbps"),
                (None, None) => "No data yet".to_owned(),
            }
        });
    }

    /// Paint the gateway's state into the Xproxy submenu.
    pub fn update_gateway(&self, status: Option<&Status>, models: usize) {
        let Some(status) = status else {
            self.xp_status.set_text("Starting…");
            self.xp_detail.set_text("");
            self.xp_toggle.set_text("Pause");
            for item in [&self.xp_toggle, &self.xp_url, &self.xp_key, &self.xp_refresh] {
                item.set_enabled(false);
            }
            return;
        };

        if status.running {
            self.xp_status
                .set_text(&format!("Serving · {}", status.base_url.as_deref().unwrap_or("")));
            self.xp_detail.set_text(&format!(
                "{models} models · {}",
                if status.secrets_sealed {
                    "credentials sealed"
                } else {
                    "credentials unsealed"
                }
            ));
            self.xp_toggle.set_text("Pause");
        } else if let Some(error) = &status.error {
            self.xp_status.set_text(&format!("Stopped · {error}"));
            self.xp_detail.set_text("");
            self.xp_toggle.set_text("Start");
        } else {
            self.xp_status.set_text("Paused");
            self.xp_detail.set_text(&format!("{models} models"));
            self.xp_toggle.set_text("Start");
        }

        for item in [&self.xp_toggle, &self.xp_url, &self.xp_key, &self.xp_refresh] {
            item.set_enabled(true);
        }
        // Copying needs something to copy.
        self.xp_url.set_enabled(status.running);
        self.xp_key.set_enabled(status.running);
        self.xp_refresh.set_enabled(status.running);
    }

    /// Act on a menu click. Returns `true` when the user asked to quit.
    pub fn handle_menu(
        &self,
        id: &MenuId,
        control: &pingbar::Control,
        gateway: &GatewayHandle,
    ) -> bool {
        let same = |item: &dyn IsMenuItem| id == item.id();

        if same(&self.quit) {
            return true;
        }
        if same(&self.net_test) {
            control.test_speed_now();
            return false;
        }
        for (index, target) in self.targets.iter().enumerate() {
            if same(target) {
                control.set_target(ip(PRESETS[index].1));
                for (other, item) in self.targets.iter().enumerate() {
                    item.set_checked(other == index);
                }
                return false;
            }
        }
        if same(&self.xp_toggle) {
            gateway.toggle();
        } else if same(&self.xp_url) {
            gateway.copy(Copy::BaseUrl);
        } else if same(&self.xp_key) {
            gateway.copy(Copy::Key);
        } else if same(&self.xp_refresh) {
            gateway.refresh_tokens();
        }
        false
    }
}

/// What the capsule says: one number, and a glyph when the gateway is off.
fn capsule_title(snapshot: &Snapshot) -> String {
    if let Some(_error) = &snapshot.error {
        return "err".to_owned();
    }
    if snapshot.samples == 0 {
        return "…".to_owned();
    }
    if !snapshot.online() {
        return "RTO".to_owned();
    }
    match snapshot.ping_ms {
        Some(ping) => format!("{ping:.0} ms"),
        None => "—".to_owned(),
    }
}
