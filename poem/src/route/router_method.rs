use std::{future::Future, marker::PhantomData};

use futures_util::{FutureExt, future::Either};

use crate::{
    Endpoint, EndpointExt, IntoEndpoint, Request, Response, Result, endpoint::BoxEndpoint,
    error::MethodNotAllowedError, http::Method,
};

/// Routing object for HTTP methods with compile-time state verification.
///
/// `RouteMethod<S>` requires state of type `S` to handle requests, similar to `Route<S>`.
///
/// # Errors
///
/// - [`MethodNotAllowedError`]
///
/// # Example
///
/// ```
/// use poem::{
///     Endpoint, Request, RouteMethod, handler,
///     http::{Method, StatusCode},
/// };
///
/// #[handler]
/// fn handle_get() -> &'static str {
///     "get"
/// }
///
/// #[handler]
/// fn handle_post() -> &'static str {
///     "post"
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let route_method = RouteMethod::<()>::new().get(handle_get).post(handle_post);
///
/// let resp = route_method
///     .get_response(Request::builder().method(Method::GET).finish())
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "get");
///
/// let resp = route_method
///     .get_response(Request::builder().method(Method::POST).finish())
///     .await;
/// assert_eq!(resp.status(), StatusCode::OK);
/// assert_eq!(resp.into_body().into_string().await.unwrap(), "post");
///
/// let resp = route_method
///     .get_response(Request::builder().method(Method::PUT).finish())
///     .await;
/// assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
/// # });
/// ```
pub struct RouteMethod<S = ()> {
    methods: Vec<(Method, BoxEndpoint<'static, S>)>,
    _marker: PhantomData<S>,
}

impl<S> Default for RouteMethod<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> RouteMethod<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create a `RouteMethod` object.
    pub fn new() -> Self {
        RouteMethod {
            methods: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Sets the endpoint for the specified `method`.
    #[must_use]
    pub fn method<E>(mut self, method: Method, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.methods
            .push((method, ep.into_endpoint().map_to_response().boxed()));
        self
    }

    /// Sets the endpoint for `GET`.
    #[must_use]
    pub fn get<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::GET, ep)
    }

    /// Sets the endpoint for `POST`.
    #[must_use]
    pub fn post<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::POST, ep)
    }

    /// Sets the endpoint for `PUT`.
    #[must_use]
    pub fn put<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::PUT, ep)
    }

    /// Sets the endpoint for `DELETE`.
    #[must_use]
    pub fn delete<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::DELETE, ep)
    }

    /// Sets the endpoint for `HEAD`.
    #[must_use]
    pub fn head<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::HEAD, ep)
    }

    /// Sets the endpoint for `OPTIONS`.
    #[must_use]
    pub fn options<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::OPTIONS, ep)
    }

    /// Sets the endpoint for `CONNECT`.
    #[must_use]
    pub fn connect<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::CONNECT, ep)
    }

    /// Sets the endpoint for `PATCH`.
    #[must_use]
    pub fn patch<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::PATCH, ep)
    }

    /// Sets the endpoint for `TRACE`.
    #[must_use]
    pub fn trace<E>(self, ep: E) -> Self
    where
        E: IntoEndpoint<S>,
        E::Endpoint: 'static,
    {
        self.method(Method::TRACE, ep)
    }
}

impl<S> Endpoint<S> for RouteMethod<S>
where
    S: Clone + Send + Sync + 'static,
{
    type Output = Response;

    fn call(&self, mut req: Request, state: &S) -> impl Future<Output = Result<Self::Output>> + Send {
        let state_clone = state.clone();
        match self
            .methods
            .iter()
            .find(|(method, _)| method == req.method())
            .map(|(_, ep)| ep)
        {
            Some(ep) => Either::Left(ep.call(req, state)),
            None => {
                if req.method() == Method::HEAD {
                    Either::Right(Either::Left(
                        async move {
                            req.set_method(Method::GET);
                            let mut resp: Response = self.call(req, &state_clone).await?;
                            resp.set_body(());
                            Ok(resp)
                        }
                        .boxed(),
                    ))
                } else {
                    Either::Right(Either::Right(async { Err(MethodNotAllowedError.into()) }))
                }
            }
        }
    }
}

/// A helper function, similar to `RouteMethod::new().get(ep)`.
pub fn get<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().get(ep)
}

/// A helper function, similar to `RouteMethod::new().post(ep)`.
pub fn post<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().post(ep)
}

/// A helper function, similar to `RouteMethod::new().put(ep)`.
pub fn put<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().put(ep)
}

/// A helper function, similar to `RouteMethod::new().delete(ep)`.
pub fn delete<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().delete(ep)
}

/// A helper function, similar to `RouteMethod::new().head(ep)`.
pub fn head<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().head(ep)
}

/// A helper function, similar to `RouteMethod::new().options(ep)`.
pub fn options<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().options(ep)
}

/// A helper function, similar to `RouteMethod::new().connect(ep)`.
pub fn connect<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().connect(ep)
}

/// A helper function, similar to `RouteMethod::new().patch(ep)`.
pub fn patch<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().patch(ep)
}

/// A helper function, similar to `RouteMethod::new().trace(ep)`.
pub fn trace<E, S>(ep: E) -> RouteMethod<S>
where
    E: IntoEndpoint<S>,
    E::Endpoint: 'static,
    S: Clone + Send + Sync + 'static,
{
    RouteMethod::new().trace(ep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{handler, http::StatusCode, test::TestClient};

    #[tokio::test]
    async fn method_not_allowed() {
        let resp = TestClient::new(RouteMethod::<()>::new()).get("/").send().await;
        resp.assert_status(StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn route_method() {
        #[handler(internal)]
        fn index() -> &'static str {
            "hello"
        }

        for method in &[
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::PATCH,
            Method::TRACE,
        ] {
            let route: RouteMethod<()> = RouteMethod::new().method(method.clone(), index).post(index);
            let resp = TestClient::new(route)
                .request(method.clone(), "/")
                .send()
                .await;
            resp.assert_status_is_ok();
            resp.assert_text("hello").await;
        }

        macro_rules! test_method {
            ($(($id:ident, $method:ident)),*) => {
                $(
                let route: RouteMethod<()> = RouteMethod::new().$id(index).post(index);
                let resp = TestClient::new(route).request(Method::$method, "/").send().await;
                resp.assert_status_is_ok();
                resp.assert_text("hello").await;
                )*
            };
        }

        test_method!(
            (get, GET),
            (post, POST),
            (delete, DELETE),
            (put, PUT),
            (head, HEAD),
            (options, OPTIONS),
            (connect, CONNECT),
            (patch, PATCH),
            (trace, TRACE)
        );
    }

    #[tokio::test]
    async fn head_method() {
        #[handler(internal)]
        fn index() -> &'static str {
            "hello"
        }

        let route: RouteMethod<()> = RouteMethod::new().get(index);
        let resp = TestClient::new(route).head("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("").await;
    }
}
