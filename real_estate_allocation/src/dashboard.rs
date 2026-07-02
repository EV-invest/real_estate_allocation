use dioxus::prelude::*;
use dockview_dioxus::{Config, DockPanel, Group, GroupId, Keybind, MinSize, PackedApi, PackedArea, PanelId, Step};

use crate::{
	api::load_default_layout,
	map::MapPanel,
	panels::{ChartPanel, DetailsPanel, LotsPanel, MediaPanel, PortfolioHeatmap, TopBar},
};

#[component]
pub fn Dashboard() -> Element {
	let panels = use_signal(|| {
		vec![
			DockPanel {
				id: PanelId("map".into()),
				title: "Map".into(),
				content: rsx! { MapPanel {} },
			},
			DockPanel {
				id: PanelId("media".into()),
				title: "Media".into(),
				content: rsx! { MediaPanel {} },
			},
			DockPanel {
				id: PanelId("chart".into()),
				title: "Chart".into(),
				content: rsx! { ChartPanel {} },
			},
			DockPanel {
				id: PanelId("heatmap".into()),
				title: "Portfolio".into(),
				content: rsx! { PortfolioHeatmap {} },
			},
			DockPanel {
				id: PanelId("lots".into()),
				title: "Lots".into(),
				content: rsx! { LotsPanel {} },
			},
			DockPanel {
				id: PanelId("details".into()),
				title: "Details".into(),
				content: rsx! { DetailsPanel {} },
			},
		]
	});

	// `PackedArea` hands us its imperative handle once, after the first measure. Stash it so the
	// load-or-seed effect and the save shortcut can both drive the grid from outside that callback.
	let mut api_handle = use_signal(|| None::<PackedApi>);
	let on_ready = Callback::new(move |api: PackedApi| api_handle.set(Some(api)));

	// Fetch and restore the band group's saved seed: once the grid handle arrives (post first
	// measure, so the band is classified), and again whenever a resize crosses into another
	// group — each group points at its own arrangement. Falls back to the built-in one
	// (map+media tabbed, the rest packed beside). Unsaved tweaks are dropped on a group
	// switch; `s` is what persists them. The group lands in the shared `SeedGroup` so the
	// build tag can show which seed is live.
	let mut loaded_group = use_context::<crate::app::SeedGroup>();
	let min = MinSize::Steps { w: Step(2), h: Step(2) };
	use_effect(move || {
		let Some(api) = api_handle() else { return };
		let bp = *api.breakpoint.read();
		let group = crate::api::seed_key(bp);
		if *loaded_group.peek() == Some(group) {
			return;
		}
		loaded_group.set(Some(group));

		spawn(async move {
			let mut api = api;
			let seed = |api: &mut PackedApi| {
				// A group switch lands on a populated grid; the built-in seed replaces it wholesale.
				*api.grid.write() = Default::default();
				let map_group = {
					let id = api.grid.write().mint_group_id();
					Group {
						id,
						tabs: vec![PanelId("map".into()), PanelId("media".into())],
						active: 0,
					}
				};
				api.place(map_group, 4, 3, min);
				for panel in ["chart", "heatmap", "lots", "details"] {
					let group = Group::new(api.grid.write().mint_group_id(), PanelId(panel.into()));
					api.place(group, 4, 3, min);
				}
			};

			let result = load_default_layout(bp).await;
			// A later group switch superseded this fetch while it was in flight.
			if *loaded_group.peek() != Some(group) {
				return;
			}
			match result {
				Ok(Some(json)) => {
					if let Err(e) = api.load(&json) {
						// A corrupt saved layout must not blank the dashboard; log it and seed the default.
						dioxus::logger::tracing::error!(?e, "saved layout corrupt — using built-in seed");
						seed(&mut api);
					}
				}
				Ok(None) => seed(&mut api),
				Err(e) => {
					// A fetch failure for the optional default likewise degrades to the built-in seed.
					dioxus::logger::tracing::error!(%e, "load_default_layout failed — using built-in seed");
					seed(&mut api);
				}
			}
		});
	});

	// `s` → save the live arrangement as its band group's global seed (an xl/lg save doubles as
	// the `default`), registered as a `PackedArea` host action. The closure gets the same
	// `PackedApi` `on_ready` handed us and POSTs its serialized grid; only fires browser-side
	// (the listener is wasm-only) but compiles everywhere. The toast gives the user feedback
	// that the save landed (or didn't), auto-clearing after a beat.
	let mut toast = use_signal(|| None::<String>);
	let save_layout = Callback::new(move |api: PackedApi| {
		let json = api.save();
		let bp = *api.breakpoint.peek();
		spawn(async move {
			let msg = match crate::api::save_default_layout(json, bp).await {
				Ok(()) => match crate::api::seed_key(bp) {
					"xl" => "Layout saved (xl/lg + default)".to_string(),
					key => format!("Layout saved ({key})"),
				},
				Err(e) => {
					dioxus::logger::tracing::error!(%e, "save default layout failed");
					"Save failed".to_string()
				}
			};
			toast.set(Some(msg));
			#[cfg(target_arch = "wasm32")]
			{
				gloo_timers::future::TimeoutFuture::new(2500).await;
				toast.set(None);
			}
		});
	});
	let config = Config {
		actions: vec![(Keybind { key: "s", alt: false, ctrl: false }, save_layout)],
		..Default::default()
	};

	// The packed grid's `+` ("add window as a tab") button asks the host to open a tab in `group`.
	// This dashboard has a fixed panel set with nothing to spawn, so the button is inert.
	// ponytail: wire to a panel picker if/when runtime windows exist.
	use_context_provider(|| Callback::new(|_group: GroupId| {}));

	rsx! {
		div { class: "flex h-screen flex-col bg-background text-foreground",
			TopBar {}
			div { class: "relative min-h-0 flex-1",
				PackedArea { panels, on_ready: Some(on_ready), config: Some(config) }
			}
			if let Some(msg) = toast() {
				div {
					class: "pointer-events-none fixed bottom-6 left-1/2 z-50 -translate-x-1/2 rounded
						border border-main-mist/20 bg-main-black/90 px-4 py-2 font-mono text-xs
						tracking-wider text-main-accent-t1 shadow-lg",
					"{msg}"
				}
			}
		}
	}
}
