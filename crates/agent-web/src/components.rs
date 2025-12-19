//! UI Components

use leptos::prelude::*;
use crate::api::ChatMessage;

/// Message bubble component
#[component]
pub fn MessageBubble(message: ChatMessage) -> impl IntoView {
    let class = format!("message message-{}", message.role);
    
    view! {
        <div class=class>
            <span class="role">{message.role.clone()}</span>
            <p class="content">{message.content.clone()}</p>
        </div>
    }
}
