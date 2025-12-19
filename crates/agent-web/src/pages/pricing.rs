//! Pricing Page

use leptos::prelude::*;
use crate::api;

#[component]
pub fn PricingPage() -> impl IntoView {
    let checkout = move |plan: &str| {
        let plan = plan.to_string();
        leptos::task::spawn_local(async move {
            if let Ok(url) = api::create_checkout(&plan).await {
                if let Some(window) = web_sys::window() {
                    let _ = window.location().set_href(&url);
                }
            }
        });
    };

    view! {
        <div class="pricing">
            <h1>"Pricing"</h1>
            <p class="subtitle">"Simple plans for local AI"</p>

            <div class="plans">
                <div class="plan">
                    <h2>"Free"</h2>
                    <div class="price">"$0"<span>"/month"</span></div>
                    <ul>
                        <li>"50 requests/day"</li>
                        <li>"Basic tools"</li>
                    </ul>
                    <a href="/chat" class="btn">"Get Started"</a>
                </div>

                <div class="plan featured">
                    <span class="badge">"Popular"</span>
                    <h2>"Pro"</h2>
                    <div class="price">"$29"<span>"/month"</span></div>
                    <ul>
                        <li>"Unlimited requests"</li>
                        <li>"All tools"</li>
                        <li>"Priority support"</li>
                    </ul>
                    <button class="btn btn-primary" on:click=move |_| checkout("pro")>
                        "Subscribe"
                    </button>
                </div>

                <div class="plan">
                    <h2>"Team"</h2>
                    <div class="price">"$99"<span>"/month"</span></div>
                    <ul>
                        <li>"Everything in Pro"</li>
                        <li>"5 seats"</li>
                        <li>"API access"</li>
                    </ul>
                    <button class="btn" on:click=move |_| checkout("team")>
                        "Subscribe"
                    </button>
                </div>
            </div>
        </div>
    }
}
