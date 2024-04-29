use std::future::IntoFuture;

use lazy_static::lazy_static;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    hooks::use_params,
    params::Params,
    ParamSegment, SsrMode, StaticSegment,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/ssr_modes.css"/>
        <Title text="Welcome to Leptos"/>
        <Meta name="color-scheme" content="dark light"/>
        <Router>
            <main>
                // When you're using nested routing, you'll use <Routes>
                // <FlatRoutes> just offers a small optimization when you have a "flat" route tree
                // like this, i.e., when there are not actually any nested routes
                <FlatRoutes fallback>
                    // We’ll load the home page with out-of-order streaming and <Suspense/>
                    // 
                    // I'm planning to have something like a path!("/") macro that can create these
                    // tuples for you, I just haven't done it yet :-)
                    <Route path=StaticSegment("") view=HomePage/>

                    // We'll load the posts with async rendering, so they can set
                    // the title and metadata *after* loading the data
                    <Route
                        path=(StaticSegment("post"), ParamSegment("id"))
                        view=Post
                        ssr=SsrMode::Async
                    />
                    <Route
                        path=(StaticSegment("post_in_order"), ParamSegment("id"))
                        view=Post
                        ssr=SsrMode::InOrder
                    />
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    // a variety of different encodings can now be specified per Resource
    // it defaults to a FromStr/Display-based encoding -- maybe serde_json should be the default,
    // but it's not for now 
    // 
    // this could be Resource::new_rkyv() etc. (the encodings are enabled by different feature
    // flags on leptos_server)
    let posts = Resource::new_serde(|| (), |_| list_post_metadata());

    // Suspense now works directly with individual Futures 
    // you can Suspend one or more Futures within an Suspense component
    //
    // in 0.1-0.6, Suspense runs its whole body once initially to register resource reads, and so
    // you need to check whether a resource is None 
    // 
    // in 0.7, you can just .await the actual value of the resource
    let posts_view = move || Suspend(async move {
        posts.await.map(|posts| {
            posts.into_iter()
                .map(|post| view! {
                    <li>
                        <a href=format!("/post/{}", post.id)>{post.title.clone()}</a>
                        "|"
                        <a href=format!("/post_in_order/{}", post.id)>{post.title} "(in order)"</a>
                    </li>
                })
                .collect::<Vec<_>>()
        })
    });

    // resources can wait for values from other resources
    // TBH I'm not sure whether this needs to explicitly track `posts` (I think it does, with the
    // way hydration of resources is at the moment). Feel free to play around.
    let post_count = Resource::new(move || posts.track(), move |_| async move { 
        posts.await.unwrap_or_default().len()
    });

    view! {
        <h1>"My Great Blog"</h1>
        <Suspense fallback=move || view! { <p>"Loading posts..."</p> }>
            // into_future() allows us to turn the `post_count` Resource into a Future without an
            // async block
            <p>"Found " {Suspend(post_count.into_future())} " posts."</p>
            <ul>{posts_view}</ul>
        </Suspense>
    }
}

#[derive(Params, Copy, Clone, Debug, PartialEq, Eq)]
pub struct PostParams {
    id: Option<usize>,
}

#[component]
fn Post() -> impl IntoView {
    let query = use_params::<PostParams>();
    let id = move || {
        // you can use .read() to get a read-lock on a signal's value, 
        // which can help reduce the nesting of .with()
        query.read().as_ref()
            .map(|q| q.id.unwrap_or_default())
            .map_err(|_| PostError::InvalidId)
    };
    let post_resource = Resource::new_serde(id, |id| async move {
        match id {
            Err(e) => Err(e),
            Ok(id) => get_post(id)
                .await
                .map(|data| data.ok_or(PostError::PostNotFound))
                .map_err(|_| PostError::ServerError),
        }
    });

    let post_view = Suspend(async move {
        match post_resource.await {
            Ok(Ok(post)) => Ok(view! {
                <h1>{post.title.clone()}</h1>
                <p>{post.content.clone()}</p>

                // since we're using async rendering for this page,
                // this metadata should be included in the actual HTML <head>
                // when it's first served
                <Title text=post.title/>
                <Meta name="description" content=post.content/>
            }),
            _ => Err(PostError::ServerError),
        }
    });

    view! {
        <em>"The world's best content."</em>
        <Suspense fallback=move || view! { <p>"Loading post..."</p> }>
            <ErrorBoundary fallback=|errors| {
                view! {
                    // You'll notice the type of `errors` here is ArcRwSignal, and it's Clone but not
                    // Copy. We only need to use it once, so this is fine, but if we needed it multiple
                    // times and wanted a Copy handle, we can easily get that like this
                    // 
                    // let errors = RwSignal::from(errors);
                    // 
                    // All of the signal types now have Arc variants, and the Copy variants are built
                    // on top of those
                    <div class="error">
                        <h1>"Something went wrong."</h1>
                        <ul>
                            {move || {
                                errors
                                    .get()
                                    .into_iter()
                                    .map(|(_, error)| view! { <li>{error.to_string()}</li> })
                                    .collect::<Vec<_>>()
                            }}

                        </ul>
                    </div>
                }
            }>{post_view}</ErrorBoundary>
        </Suspense>
    }
}

// Dummy API
lazy_static! {
    static ref POSTS: Vec<Post> = vec![
        Post {
            id: 0,
            title: "My first post".to_string(),
            content: "This is my first post".to_string(),
        },
        Post {
            id: 1,
            title: "My second post".to_string(),
            content: "This is my second post".to_string(),
        },
        Post {
            id: 2,
            title: "My third post".to_string(),
            content: "This is my third post".to_string(),
        },
    ];
}

#[derive(Error, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostError {
    #[error("Invalid post ID.")]
    InvalidId,
    #[error("Post not found.")]
    PostNotFound,
    #[error("Server error.")]
    ServerError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Post {
    id: usize,
    title: String,
    content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostMetadata {
    id: usize,
    title: String,
}

#[server]
pub async fn list_post_metadata() -> Result<Vec<PostMetadata>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(POSTS
        .iter()
        .map(|data| PostMetadata {
            id: data.id,
            title: data.title.clone(),
        })
        .collect())
}

#[server]
pub async fn get_post(id: usize) -> Result<Option<Post>, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok(POSTS.iter().find(|post| post.id == id).cloned())
}
