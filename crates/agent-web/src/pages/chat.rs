//! Chat Page

use leptos::prelude::*;
use crate::api;
use crate::components::MessageBubble;

#[component]
pub fn ChatPage() -> impl IntoView {
    let (messages, set_messages) = signal(Vec::<api::ChatMessage>::new());
    let (input, set_input) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (license_key, set_license_key) = signal(String::new());

    let send = move |_| {
        let msg = input.get();
        if msg.is_empty() || loading.get() {
            return;
        }

        // Add user message
        set_messages.update(|msgs| {
            msgs.push(api::ChatMessage {
                role: "user".into(),
                content: msg.clone(),
            });
        });

        set_input.set(String::new());
        set_loading.set(true);

        let key = license_key.get();
        leptos::task::spawn_local(async move {
            match api::send_chat(&msg, if key.is_empty() { None } else { Some(&key) }).await {
                Ok(response) => {
                    set_messages.update(|msgs| {
                        msgs.push(api::ChatMessage {
                            role: "assistant".into(),
                            content: response,
                        });
                    });
                }
                Err(e) => {
                    set_messages.update(|msgs| {
                        msgs.push(api::ChatMessage {
                            role: "error".into(),
                            content: e,
                        });
                    });
                }
            }
            set_loading.set(false);
        });
    };

    view! {
        <div class="chat">
            <aside class="sidebar">
                <h2>"Settings"</h2>
                <div class="field">
                    <label>"License Key"</label>
                    <input
                        type="text"
                        placeholder="XXXX-XXXX-XXXX-XXXX"
                        prop:value=move || license_key.get()
                        on:input=move |ev| set_license_key.set(event_target_value(&ev))
                    />
                </div>
            </aside>

            <main class="chat-main">
                <div class="messages">
                    <For
                        each=move || messages.get()
                        key=|msg| format!("{}-{}", msg.role, msg.content.len())
                        children=move |msg| view! { <MessageBubble message=msg /> }
                    />
                    <Show when=move || loading.get()>
                        <div class="message loading">"..."</div>
                    </Show>
                </div>

                <div class="input-area">
                    <textarea
                        placeholder="Ask anything..."
                        prop:value=move || input.get()
                        on:input=move |ev| set_input.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" && !ev.shift_key() {
                                ev.prevent_default();
                                send(());
                            }
                        }
                    />
                    <button on:click=send disabled=move || loading.get()>
                        {move || if loading.get() { "..." } else { "Send" }}
                    </button>
                </div>
            </main>
        </div>
    }
}
