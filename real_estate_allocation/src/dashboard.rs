use dioxus::prelude::*;
use dockview_dioxus::{Config, DockPanel, Group, GroupId, Keybind, MinSize, PackedApi, PackedArea, PanelId, Step};

use crate::{
	api::load_default_layout,
	map::MapPanel,
	panels::{ChartPanel, DetailsPanel, MediaPanel, PortfolioHeatmap, TopBar},
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

	let layout = use_resource(|| async move { load_default_layout().await });

	// First time both the grid handle and the layout fetch are ready: restore the saved global
	// default, else fall back to the built-in arrangement (map+media tabbed, the rest packed beside).
	let mut applied = use_signal(|| false);
	let min = MinSize::Steps { w: Step(2), h: Step(2) };
	use_effect(move || {
		if applied() {
			return;
		}
		let Some(mut api) = api_handle() else { return };
		let resolved = layout.read();
		let Some(result) = resolved.as_ref() else { return };

		let seed = |api: &mut PackedApi| {
			let map_group = {
				let id = api.grid.write().mint_group_id();
				Group {
					id,
					tabs: vec![PanelId("map".into()), PanelId("media".into())],
					active: 0,
				}
			};
			api.place(map_group, 4, 3, min);
			for panel in ["chart", "heatmap", "details"] {
				let group = Group::new(api.grid.write().mint_group_id(), PanelId(panel.into()));
				api.place(group, 4, 3, min);
			}
		};

		match result {
			Ok(Some(json)) => {
				if let Err(e) = api.load(json) {
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
		applied.set(true);
	});

	// Alt+S → save the live arrangement as the global default, registered as a `PackedArea` host
	// action. The closure gets the same `PackedApi` `on_ready` handed us and POSTs its serialized
	// grid; only fires browser-side (the listener is wasm-only) but compiles everywhere.
	let save_layout = Callback::new(|api: PackedApi| {
		let json = api.save();
		spawn(async move {
			if let Err(e) = crate::api::save_default_layout(json).await {
				dioxus::logger::tracing::error!(%e, "save default layout failed");
			}
		});
	});
	let config = Config {
		actions: vec![(Keybind { key: "s", alt: true, ctrl: false }, save_layout)],
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
		}
	}
}
