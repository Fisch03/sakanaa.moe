use crate::components::*;
use maud::{html, Markup};

pub fn hardware() -> Markup {
    section(
        "hardware",
        html! {
            table {
                tr {
                    th {}
                    th { "Reimu (Main PC)" }
                    th { "Nitori (Server)" }
                    th { "Sakuya (Secondary Server) " }
                }
                tr {
                    td { "CPU" }
                    td { "Ryzen 7 3700X" }
                    td { "Ryzen 5 5600G" }
                    td { "Intel Xeon X5660" }
                }
                tr {
                    td { "GPU" }
                    td { "Nvidia RTX 3080" }
                    td { "iGPU" }
                    td { "no haha" }
                }
                tr {
                    td { "Storage" }
                    td { "256GB NVME SSD" br; "2TB SATA SSD" br; "1TB HDD" }
                    td { "128GB NVME SSD (as cache)" br; "12TB HDD (3x4TB)" }
                    td { "4TB HDD" }
                }
            }
            p {
                "other important members of the family: "
                ul {
                    li { "Flandre - Thinkpad P50" }
                    li { "Remilia - Steam Deck" }
                }
            }
        },
        &SectionConfig {
            id: Some("Hardware"),
            ..Default::default()
        },
    )
}
