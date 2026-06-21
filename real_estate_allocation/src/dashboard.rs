use dioxus::prelude::*;
use dockview_dioxus::{DockApi, DockArea, DockPanel, PanelId, Position};

use crate::{
	map::MapPanel,
	panels::{ChartPanel, DetailsPanel, MediaPanel, PortfolioHeatmap, TopBar},
};

#[component]
pub fn Dashboard() -> Element {
	let panels = vec![
		DockPanel { id: PanelId("map".into()), title: "Map".into(), content: rsx! { MapPanel {} } },
		DockPanel { id: PanelId("chart".into()), title: "Chart".into(), content: rsx! { ChartPanel {} } },
		DockPanel { id: PanelId("heatmap".into()), title: "Portfolio".into(), content: rsx! { PortfolioHeatmap {} } },
		DockPanel { id: PanelId("media".into()), title: "Media".into(), content: rsx! { MediaPanel {} } },
		DockPanel { id: PanelId("details".into()), title: "Details".into(), content: rsx! { DetailsPanel {} } },
	];
	// Order = stable overlay render order; do NOT reorder later (it remounts panels).

	// Seed an arrangement on first load that roughly mirrors the old layout; fires once,
	// only when there's no saved layout. Same code path a real drag uses.
	//   ┌ map+media ┬ chart   ┐
	//   ├ heatmap   ┴ details ┘
	let seed = Callback::new(move |mut api: DockApi| {
		api.move_panel(PanelId("chart".into()), vec![], Position::Right);
		api.move_panel(PanelId("details".into()), vec![1], Position::Bottom);
		api.move_panel(PanelId("heatmap".into()), vec![0], Position::Bottom);
	});

	rsx! {
		div { class: "flex h-screen flex-col bg-background text-foreground",
			TopBar {}
			div { class: "relative min-h-0 flex-1",
				DockArea {
					panels,
					storage_key: Some("rea-dashboard-layout".to_string()),
					on_ready: Some(seed),
				}
			}
		}
	}
}
