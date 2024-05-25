use std::future::IntoFuture;

use leptos::{
    prelude::*,
    tachys::view::any_view::{AnyView, IntoAny},
};
use leptos_router::{components::*, Lazy, LazyRoute, Outlet, StaticSegment};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[cfg(feature = "split")]
use wasm_split::wasm_split;

#[wasm_bindgen]
pub fn main() {
    console_error_panic_hook::set_once();

    /*fmt()
        .with_writer(
            // To avoide trace events in the browser from showing their
            // JS backtrace, which is very annoying, in my opinion
            MakeConsoleWriter::default().map_trace_level_to(tracing::Level::DEBUG),
        )
        // For some reason, if we don't do this in the browser, we get
        // a runtime error.
        .without_time()
        .init();

    tracing::info!("here in tracing!");*/

    leptos::mount::mount_to_body(|| {
        let count = RwSignal::new(0);
        provide_context(count);

        view! {
            <Router>
                <a href="/">"A"</a>
                <a href="/b">"B"</a>
                <a href="/c">"C"</a>
                <Routes fallback=|| "Not found.">
                    <Route path=StaticSegment("") view=ViewA/>
                    <ParentRoute path=StaticSegment("") view={Lazy::<ViewB>::new()}>
                        <Route path=StaticSegment("b") view={Lazy::<ViewBChild>::new()}/>
                    </ParentRoute>
                    <Route path=StaticSegment("c") view={Lazy::<ViewC>::new()}/>
                </Routes>
            </Router>
        }
    });
}

// View A: A plain old synchronous route, just like they all currently work. The WASM binary code
// for this is shipped as part of the main bundle.  Any data-loading code (like resources that run
// in the body of the component) will be shipped as part of the main bundle.

#[component]
pub fn ViewA() -> impl IntoView {
    view! { <p>"View A"</p> }
}

// View B: A nested parent-child, each of which is lazy. The code for each of these is loaded in
// parallel when we navigate to /b.
#[derive(Debug, Clone)]
pub struct ViewB;

// Lazy-loaded routes need to implement the LazyRoute trait. They define a "route data" struct,
// which is created with `::data()`, and then a separate view function which is lazily loaded.
//
// This is important because it allows us to concurrently 1) load the route data, and 2) lazily
// load the component, rather than creating a "waterfall" where we can't start loading the route
// data until we've received the view.
impl LazyRoute<Dom> for ViewB {
    fn data() -> Self {
        Self
    }

    // This is a bunch of boilerplate, which can be turned into a macro pretty easily.
    async fn view(self) -> AnyView<Dom> {
        #[cfg_attr(feature = "split", wasm_split::wasm_split(view_b))]
        async fn view(this: ViewB) -> AnyView<Dom> {
            view! {
                <p>"View B"</p>
                <hr/>
                <Outlet/>
            }
            .into_any()
        }

        view(self).await
    }
}

#[derive(Clone, Debug)]
pub struct ViewBChild {
    data: AsyncDerived<String>,
}

impl LazyRoute<Dom> for ViewBChild {
    fn data() -> Self {
        Self {
            data: AsyncDerived::new_unsync(|| async {
                gloo_net::http::Request::get("https://jsonplaceholder.typicode.com/todos/1")
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap()
            }),
        }
    }

    // Note that the view here takes the route data as its argument, which means you have
    // fully-typed access to the route data, in the view.
    async fn view(self) -> AnyView<Dom> {
        #[cfg_attr(feature = "split", wasm_split::wasm_split(view_b_child))]
        async fn view(this: ViewBChild) -> AnyView<Dom> {
            view! {
                <p>"Nested Child"</p>
                <Suspense fallback=|| "Loading...">
                    <pre>{Suspend(this.data.into_future())}</pre>
                </Suspense>
            }
            .into_any()
        }

        view(self).await
    }
}

// View C: A nested parent-child, each of which is lazy, and where the (deserialization-heavy)
// data-loading function for the child is *also* lazy-loaded.
#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    #[serde(rename = "postId")]
    post_id: usize,
    id: usize,
    name: String,
    email: String,
    body: String,
}

#[cfg_attr(feature = "split", wasm_split::wasm_split(deserialize_comments))]
fn deserialize_comments(data: &str) -> Vec<Comment> {
    serde_json::from_str(data).unwrap()
}

#[derive(Clone, Debug)]
pub struct ViewC {
    data: AsyncDerived<Vec<Comment>>,
}

impl LazyRoute<Dom> for ViewC {
    fn data() -> Self {
        Self {
            data: AsyncDerived::new_unsync(|| async move {
                let data =
                    gloo_net::http::Request::get("https://jsonplaceholder.typicode.com/comments")
                        .send()
                        .await
                        .unwrap()
                        .text()
                        .await
                        .unwrap();
                deserialize_comments(&data).await
            }),
        }
    }

    async fn view(self) -> AnyView<Dom> {
        #[cfg_attr(feature = "split", wasm_split::wasm_split(view_c))]
        async fn view(this: ViewC) -> AnyView<Dom> {
            view! {
                <p>"Nested Child"</p>
                <Suspense fallback=|| "Loading...">
                    <pre>{Suspend(async move {
                        format!("{:#?}", this.data.await)
                    })}</pre>
                </Suspense>
            }
            .into_any()
        }

        view(self).await
    }
}
