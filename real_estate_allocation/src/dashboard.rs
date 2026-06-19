use dioxus::prelude::*;

use crate::{
	map::MapPanel,
	panels::{ChartPanel, DetailsPanel, MediaPanel, PortfolioHeatmap, TopBar},
};

#[component]
pub fn Dashboard() -> Element {
	rsx! {
		div { class: "min-h-screen bg-background text-foreground",
			TopBar {}
			main { class: "flex flex-col gap-6 p-6 lg:p-8",
				// Location + price chart. 5/3 split on desktop, stacked on mobile.
				div { class: "grid grid-cols-1 gap-6 lg:grid-cols-5",
					div { class: "lg:col-span-3", MapPanel {} }
					div { class: "lg:col-span-2", ChartPanel {} }
				}
				PortfolioHeatmap {}
				// Media + deal terms. Grid stretch keeps the two cards equal height.
				div { class: "grid grid-cols-1 items-stretch gap-6 lg:grid-cols-2",
					MediaPanel {}
					DetailsPanel {}
				}
			}
		}
	}
}
