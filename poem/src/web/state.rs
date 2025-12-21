//! Compile-time verified state extraction.
//!
//! This module provides a state extraction mechanism similar to axum's `State`
//! extractor.
//!
//! # Example
//!
//! ```
//! use poem::{Route, endpoint::EndpointExt, get, handler, web::State};
//!
//! #[derive(Clone)]
//! struct AppState {
//!     db_pool: String, // In real code, this would be a database pool
//! }
//!
//! #[handler]
//! fn use_state(State(state): State<AppState>) -> String {
//!     format!("Connected to: {}", state.db_pool)
//! }
//!
//! # fn main() {
//! let state = AppState {
//!     db_pool: "postgres://localhost/mydb".to_string(),
//! };
//!
//! let app = Route::new().at("/", get(use_state)).with_state(state);
//! // `app` is now ready to serve - the state has been provided
//! # }
//! ```
//!
//! # Using `FromRef` for Substate Derivation
//!
//! The [`FromRef`] trait allows you to derive substates from a parent state.
//! This is useful when you have a composite state and handlers that need
//! different parts of it:
//!
//! ```
//! use poem::{
//!     Route,
//!     endpoint::EndpointExt,
//!     get, handler,
//!     web::{FromRef, State},
//! };
//!
//! #[derive(Clone)]
//! struct AppState {
//!     db: DbState,
//!     cache: CacheState,
//! }
//!
//! #[derive(Clone)]
//! struct DbState {
//!     connection_string: String,
//! }
//!
//! #[derive(Clone)]
//! struct CacheState {
//!     url: String,
//! }
//!
//! // Allow extracting DbState from AppState
//! impl FromRef<AppState> for DbState {
//!     fn from_ref(state: &AppState) -> Self {
//!         state.db.clone()
//!     }
//! }
//!
//! // Allow extracting CacheState from AppState
//! impl FromRef<AppState> for CacheState {
//!     fn from_ref(state: &AppState) -> Self {
//!         state.cache.clone()
//!     }
//! }
//!
//! // Handlers can extract substates directly using FromRef
//! #[handler]
//! fn use_db(State(db): State<DbState>) -> String {
//!     format!("DB: {}", db.connection_string)
//! }
//!
//! #[handler]
//! fn use_cache(State(cache): State<CacheState>) -> String {
//!     format!("Cache: {}", cache.url)
//! }
//!
//! # fn main() {
//! let state = AppState {
//!     db: DbState {
//!         connection_string: "postgres://localhost".to_string(),
//!     },
//!     cache: CacheState {
//!         url: "redis://localhost".to_string(),
//!     },
//! };
//!
//! // Register the outer state - handlers extract substates via FromRef
//! let app = Route::new()
//!     .at("/db", get(use_db))
//!     .at("/cache", get(use_cache))
//!     .with_state(state);
//! # }
//! ```

use std::ops::{Deref, DerefMut};

use crate::{FromRequest, Request, RequestBody, Result};

/// Used to do reference-to-value conversions, enabling substate extraction.
///
/// This is mainly used with [`State`] to extract "substates" from a reference
/// to the main application state.
///
/// # Example
///
/// ```
/// use poem::web::FromRef;
///
/// #[derive(Clone)]
/// struct AppState {
///     db: DbState,
/// }
///
/// #[derive(Clone)]
/// struct DbState {
///     pool: String,
/// }
///
/// impl FromRef<AppState> for DbState {
///     fn from_ref(state: &AppState) -> Self {
///         state.db.clone()
///     }
/// }
/// ```
pub trait FromRef<T> {
    /// Converts to this type from a reference to the input type.
    fn from_ref(input: &T) -> Self;
}

// Any type can be extracted from itself via Clone
impl<T> FromRef<T> for T
where
    T: Clone,
{
    fn from_ref(input: &T) -> Self {
        input.clone()
    }
}

/// Extractor for state registered with
/// [`EndpointExt::with_state`](crate::endpoint::EndpointExt::with_state).
///
/// This extractor retrieves state that was previously registered using the
/// `.with_state()` method on an endpoint.
///
/// # Example
///
/// ```
/// use poem::{Route, endpoint::EndpointExt, get, handler, web::State};
///
/// #[derive(Clone)]
/// struct AppState {
///     value: i32,
/// }
///
/// #[handler]
/// fn index(State(state): State<AppState>) -> String {
///     format!("Value: {}", state.value)
/// }
///
/// # fn main() {
/// let app = Route::new()
///     .at("/", get(index))
///     .with_state(AppState { value: 42 });
/// # }
/// ```
///
/// # Substate Extraction with `FromRef`
///
/// Use [`FromRef`] to extract substates from a composite state type:
///
/// ```
/// use poem::{
///     Route,
///     endpoint::EndpointExt,
///     get, handler,
///     web::{FromRef, State},
/// };
///
/// #[derive(Clone)]
/// struct AppState {
///     inner: InnerState,
/// }
///
/// #[derive(Clone)]
/// struct InnerState {
///     value: String,
/// }
///
/// impl FromRef<AppState> for InnerState {
///     fn from_ref(state: &AppState) -> Self {
///         state.inner.clone()
///     }
/// }
///
/// // This handler extracts InnerState from the AppState via FromRef
/// #[handler]
/// fn use_inner(State(inner): State<InnerState>) -> String {
///     inner.value.clone()
/// }
///
/// // This handler extracts the full AppState
/// #[handler]
/// fn use_outer(State(app): State<AppState>) -> String {
///     format!("outer: {}", app.inner.value)
/// }
///
/// # fn main() {
/// let app_state = AppState {
///     inner: InnerState {
///         value: "hello".to_string(),
///     },
/// };
///
/// // Register the outer state - handlers extract substates via FromRef
/// let app = Route::new()
///     .at("/inner", get(use_inner))
///     .at("/outer", get(use_outer))
///     .with_state(app_state);
/// # }
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct State<S>(pub S);

impl<S> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Wrapper type used internally to store state in request extensions (for runtime state).
///
/// This is used by the legacy runtime state injection pattern.
#[derive(Clone)]
pub struct StateData<S>(pub S);

/// Implementation of `FromRequest` for `State<T>` that uses compile-time state verification.
///
/// The `T` type must implement `FromRef<S>` where `S` is the router's state type.
/// This ensures at compile time that the handler's required state can be derived
/// from the router's state.
impl<'a, OuterState, InnerState> FromRequest<'a, OuterState> for State<InnerState>
where
    InnerState: FromRef<OuterState> + Send + Sync,
    OuterState: Send + Sync,
{
    async fn from_request(
        _req: &'a Request,
        _body: &mut RequestBody,
        state: &OuterState,
    ) -> Result<Self> {
        Ok(State(InnerState::from_ref(state)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{endpoint::EndpointExt, handler, test::TestClient};

    #[derive(Clone)]
    struct TestState {
        value: i32,
    }

    #[tokio::test]
    async fn test_state_extractor() {
        #[handler(internal)]
        async fn index(State(state): State<TestState>) -> String {
            format!("{}", state.value)
        }

        let app = index.with_state(TestState { value: 42 });
        TestClient::new(app)
            .get("/")
            .send()
            .await
            .assert_status_is_ok();
    }

    // Note: test_state_extractor_missing was removed because with compile-time
    // state verification, attempting to use a handler with State<T> without
    // providing matching state is now a compile-time error, not a runtime error.
    // This is the intended behavior of the new design.

    #[derive(Clone)]
    struct OuterState {
        inner: InnerState,
    }

    #[derive(Clone)]
    struct InnerState {
        name: String,
    }

    impl FromRef<OuterState> for InnerState {
        fn from_ref(outer: &OuterState) -> Self {
            outer.inner.clone()
        }
    }

    #[tokio::test]
    async fn test_substate_extraction() {
        // For substate extraction, you can either:
        // 1. Register the substate directly with with_state
        // 2. Extract the outer state and derive the substate manually

        #[handler(internal)]
        async fn index(State(inner): State<InnerState>) -> String {
            inner.name.clone()
        }

        let outer = OuterState {
            inner: InnerState {
                name: "test".to_string(),
            },
        };

        // Register the substate derived from outer state
        let inner = InnerState::from_ref(&outer);
        let app = EndpointExt::<InnerState>::with_state(index, inner);
        let resp = TestClient::new(app).get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("test").await;
    }

    #[tokio::test]
    async fn test_outer_state_extraction() {
        // You can also extract the full outer state
        #[handler(internal)]
        async fn index(State(outer): State<OuterState>) -> String {
            outer.inner.name.clone()
        }

        let outer = OuterState {
            inner: InnerState {
                name: "test".to_string(),
            },
        };

        let app = index.with_state(outer);
        let resp = TestClient::new(app).get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("test").await;
    }
}
