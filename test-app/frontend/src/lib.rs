use yew::prelude::*;
use gloo_net::http::Request;
use test_app_shared::HelloResponse;

#[component(App)]
fn app() -> Html {
    let message = use_state(|| None::<String>);

    {
        let message = message.clone();
        use_effect_with((), move |_| {
            let message = message.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match Request::get("/api/hello")
                    .send()
                    .await
                {
                    Ok(response) => {
                        if let Ok(hello) = response.json::<HelloResponse>().await {
                            message.set(Some(hello.message));
                        }
                    }
                    Err(e) => {
                        web_sys::console::log_1(&format!("Error: {:?}", e).into());
                    }
                }
            });
            || ()
        });
    }

    html! {
        <div>
            <h1>{ "Flux WASM Builder" }</h1>
            {
                if let Some(msg) = (*message).clone() {
                    html! { <p>{ msg }</p> }
                } else {
                    html! { <p>{ "Loading..." }</p> }
                }
            }
        </div>
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
