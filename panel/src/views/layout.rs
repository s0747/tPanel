use maud::{html, Markup};

/// Generuje element `<head>` — ładuje zewnętrzne CSS i JS jako pliki statyczne
pub fn head() -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title { "iot dashboard" }

            link rel="icon" type="image/x-icon" href="./static/favicon.ico";

            link rel="stylesheet" href="./static/pico.min.css";

            link rel="stylesheet" href="./static/uPlot.min.css";

            link rel="stylesheet" href="./static/dashboard.css";

            // Datastar (statyczny, type=module)
            script type="module" src="./static/datastar.js" {}

            script src="./static/uPlot.iife.min.js" {}
        }
    }
}
