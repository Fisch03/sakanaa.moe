//! fishnet is a opinionated, performant web framework for small projects and personal websites where [htmx](https://htmx.org/) is a first class citizen.
//!
//! # overview
//! fishnet aims to provide abstractions for splitting your page into components,
//! while trying to stay as close as possible to the served content. no virtual dom, giant javascript bundles or complex build steps.
//! in doing so, fishnet makes a few core assumptions about your project:
//! - the initially served content is *(mostly)* static. you can serve dynamically rendered content, but it's not the main focus.
//! - you want to split your page into components, but don't want to deal with the complexity of a full blown frontend framework.
//! - you want full control over the html, css and javascript that is served to the client.
//! - and most importantly: you are fine with using a library that probably isn't going to get a lot of maintenance.
//!
//! # getting started
//! ## creating a page
//! to get started with fishnet, you need to create a new [`Website`](website::Website) and add a [`Page`](page::Page) to it.
//! you can then use the [`html!`](html) (see the [maud documentation](https://maud.lambda.xyz/) for more info) macro to create the content of the page.
//! ```rust,no_run
//! use fishnet::{
//!     website::Website,
//!     page::Page,
//!     html
//! };
//!
//! #[tokio::main] // feel free to use any async runtime you like
//! async fn main() {
//!     // create a new website
//!     let website = Website::new()
//!         // add a new 'home' page to the website, accessible at '/'
//!         .add_page("/", Page::new("home").with_body(|| {
//!             html! {
//!                 h1 { "Hello, World!" }
//!             }
//!         }));
//!
//!    // serve the website on port 8080
//!     website.serve(8080).await;
//! }
//! ```
//!
//! ### using components
//! lets say you've added a button to your page:
//! ```rust
//! use fishnet::{
//!    page::Page,
//!    html
//! };
//!
//! Page::new("example").with_body(|| {
//!    html! {
//!       button onclick="alert('hello!')" { "click me!" }
//!   }
//! });
//! ```
//!
//! you have now decided to reuse this button multiple times.
//! you can achieve this by creating a new component:
//! ```rust
//! use fishnet::{
//!   page::Page,
//!   // the component prelude contains a lot of useful imports for creating components
//!   component::prelude::*,
//! };
//!
//! fn my_awesome_button(label: &str, alert: &str) -> impl BuildableComponent {
//!     // convert the str references to owned strings
//!     let label = label.to_string();
//!     let alert = format!("alert('{}')", alert);
//!
//!     // create a new 'button' component
//!     component!(Button).render(move |_| {
//!         // render the button
//!         html! {
//!            button onclick=(alert) { (label) }
//!        }
//!    })
//! }
//!
//! Page::new("example").with_body(|| {
//!    // use the component
//!   html! {
//!     (c!(my_awesome_button("click me!", "hello!")))
//!     (c!(my_awesome_button("click me too!", "goodbye!")))
//!   }
//! });
//! ```
//! let's break down what's happening here:
//! the `my_awesome_button` function is a function that constructs a new component with the contents of the button.
//! since the render function may run multiple times, we need to convert the passed references to owned strings and `move` them into the closure.
//!
//! on the page itself, we use the `c!` macro to add the component to the page. this handles all the behind the scenes work of building and rendering the component, and caching it for future use.
//! in this scenario, the `my_awesome_button` function and the components render function are both run exactly once over the lifetime of the whole page, even if the page is visited multiple times.
//! (this may not always be the case, see the chapter on dynamically rendered components for more info.)
//!
//! ### components vs html! in functions (or: why do i need to use a component at all?)
//! you might be wondering why using components is better than just something like this:
//! ```rust
//! use fishnet::{
//!     page::Page,
//!     html, Markup
//! };
//!
//! fn my_awesome_button(label: &str, alert: &str) -> Markup {
//!     html! {
//!        button onclick={"alert("(alert)")"} { (label) }
//!     }
//! }
//!
//! Page::new("example").with_body(|| {
//!    html! {
//!       (my_awesome_button("click me!", "hello!"))
//!       (my_awesome_button("click me too!", "goodbye!"))
//!   }
//! });
//! ```
//! while this is indeed much simpler, and might even be desirable for such a simple case,
//! it comes with its downsides:
//! - the button is rerendered every time the page is visited, even if it doesn't change. while this is not a problem for a simple button,
//!   it can become one for more complex scenarios. a component is static by default and usually only renders once even across page visits.
//! - there is no way to add additional functionality to the button.
//!   with a component, you can easily add a script, or a endpoint that will be queried when the button is clicked.
//!
//! this means, that in the end the tradeoff is up to the use-case. using a simple function as a component is usually fine for very small components,
//! or functions that are used *within* other components. for anything bigger, or more complex, a component is usually the better choice.

pub mod component;
pub mod page;
mod routes;
pub mod website;

pub mod css;
pub mod js;

pub use fishnet_macros::css;
pub use maud::{html, Markup};
