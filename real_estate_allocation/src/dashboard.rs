use dioxus::prelude::*;

use crate::{
	map::MapPanel,
	panels::{ChartPanel, DetailsPanel, MediaPanel},
};

#[component]
pub fn Dashboard() -> Element {
	rsx! {
		div { class: "grid grid-cols-2 grid-rows-2 gap-4 h-screen p-4 bg-background text-foreground",
			MapPanel {}
			ChartPanel {}
			MediaPanel {}
			DetailsPanel {}
		}
	}
}
