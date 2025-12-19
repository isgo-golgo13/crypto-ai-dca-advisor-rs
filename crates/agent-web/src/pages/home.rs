//! Home Page

use leptos::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="home">
            <header class="hero">
                <h1>"rust-agent"</h1>
                <p class="tagline">"Local LLM-powered AI assistant built in Rust"</p>
                <div class="cta">
                    <a href="/chat" class="btn btn-primary">"Launch Agent"</a>
                    <a href="/pricing" class="btn">"View Plans"</a>
                </div>
            </header>

            <section class="features">
                <div class="feature">
                    <h3>"🔒 Private"</h3>
                    <p>"Runs locally via Ollama. Your data never leaves your machine."</p>
                </div>
                <div class="feature">
                    <h3>"🛠️ Extensible"</h3>
                    <p>"Built-in tools with easy customization. Add your own capabilities."</p>
                </div>
                <div class="feature">
                    <h3>"⚡ Fast"</h3>
                    <p>"Rust-native performance. No Python, no overhead."</p>
                </div>
            </section>
        </div>
    }
}
