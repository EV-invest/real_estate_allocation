use dioxus::prelude::*;
use dockview_dioxus::{DockPanel, Group, GroupId, MinSize, PackedApi, PackedArea, PanelId, Step};

use crate::{
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

	// Seed once after the first measure (real column count). map+media share one tabbed tile;
	// the rest each get their own. `place` skyline-packs them left→right onto the grid.
	let min = MinSize::Steps { w: Step(2), h: Step(2) };
	let seed = Callback::new(move |mut api: PackedApi| {
		let map_group = {
			let mut g = api.grid.write();
			let id = g.mint_group_id();
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
	});

	// The packed grid's `+` ("add window as a tab") button asks the host to open a tab in `group`.
	// This dashboard has a fixed panel set with nothing to spawn, so the button is inert.
	// ponytail: wire to a panel picker if/when runtime windows exist.
	use_context_provider(|| Callback::new(|_group: GroupId| {}));

	rsx! {
		div { class: "flex h-screen flex-col bg-background text-foreground",
			TopBar {}
			div { class: "relative min-h-0 flex-1",
				PackedArea { panels, on_ready: Some(seed) }
			}
		}
	}
}
