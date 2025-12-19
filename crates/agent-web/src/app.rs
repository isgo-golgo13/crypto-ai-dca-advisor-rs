//! Main App Component

use leptos::prelude::*;
use leptos_router::{components::*, path};

use crate::pages::{ChatPage, HomePage, PricingPage};

/// Root application component
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main class="app">
                <Routes fallback=|| view! { <p>"Page not found"</p> }>
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/chat") view=ChatPage />
                    <Route path=path!("/pricing") view=PricingPage />
                </Routes>
            </main>
        </Router>
    }
}
