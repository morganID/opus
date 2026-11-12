//! Opus: one capsule, two menus.
//!
//! The window-less shape every menu bar extra has: `Accessory` activation
//! policy, no Dock tile, one status item, one `tao` event loop. The network
//! monitor runs on its own thread (pingbar's), the gateway on a tokio runtime,
//! and this thread only draws.

mod gateway;
mod tray;
mod util;

use std::time::{Duration, Instant};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::MenuEvent;

use gateway::GatewayHandle;
use tray::TrayUi;

enum UserEvent {
    Menu(tray_icon::menu::MenuId),
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "opus=info,xproxy=info".into()),
        )
        .with_target(false)
        .init();

    let monitor = pingbar::Monitor::spawn_dedicated(pingbar::Config::default());
    let control = monitor.control();
    let mut snapshots = monitor.subscribe();

    let gateway = match GatewayHandle::spawn() {
        Ok(gateway) => gateway,
        Err(error) => {
            eprintln!("opus: the gateway could not be prepared: {error}");
            std::process::exit(1);
        }
    };

    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // Menu bar only: no Dock tile, no application menu. tao defaults to the
    // regular policy, which would put a Dock icon under a capsule.
    event_loop.set_activation_policy(ActivationPolicy::Accessory);
    event_loop.set_dock_visibility(false);

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = proxy.send_event(UserEvent::Menu(event.id));
    }));

    let mut ui: Option<TrayUi> = None;
    let mut started = false;
    let mut last_gateway = Instant::now() - Duration::from_secs(5);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(200));

        if let Some(tray) = ui.as_ref() {
            // The watch channel always holds the newest snapshot, so polling it
            // on the tick is enough — no per-probe event plumbing.
            if snapshots.has_changed().unwrap_or(false) {
                let snapshot = snapshots.borrow_and_update().clone();
                tray.update_network(&snapshot);
            }
            // The gateway is polled on a slower clock: its state changes when
            // somebody clicks, not sixty times a second.
            if last_gateway.elapsed() >= Duration::from_secs(1) {
                last_gateway = Instant::now();
                tray.update_gateway(gateway.status().as_ref(), gateway.models());
            }
        }

        match event {
            Event::NewEvents(StartCause::Init) if !started => {
                started = true;
                // Building the status item before the run loop is live is
                // known to misbehave on macOS.
                match TrayUi::build(&control, &gateway) {
                    Ok(tray) => ui = Some(tray),
                    Err(error) => {
                        eprintln!("opus: the status item could not be built: {error}");
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }

            Event::UserEvent(UserEvent::Menu(id)) => {
                let quit = ui
                    .as_ref()
                    .is_some_and(|tray| tray.handle_menu(&id, &control, &gateway));
                if quit {
                    ui = None;
                    *control_flow = ControlFlow::Exit;
                }
            }

            _ => {}
        }
    })
}
