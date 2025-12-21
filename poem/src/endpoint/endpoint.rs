use std::{future::Future, marker::PhantomData, sync::Arc};

use futures_util::{FutureExt, future::BoxFuture};

use super::{
    After, AndThen, Around, Before, CatchAllError, CatchError, InspectAllError, InspectError, Map,
    MapToResponse, ToResponse,
};
use crate::{
    Error, IntoResponse, Middleware, Request, Response, Result,
    error::IntoResult,
    middleware::{AddData, AddDataEndpoint},
};

/// An HTTP request handler.
///
/// The type parameter `S` represents the state type that this endpoint receives.
/// - `Endpoint<()>` (or just `Endpoint` with default) is the standard form used by handlers
/// - `Endpoint<S>` can receive state of type S via the `call` method
///
/// # State Handling
///
/// Poem uses an extractor-based approach for state. Use [`EndpointExt::with_state`]
/// to provide state that can be extracted via [`State`](crate::web::State):
///
/// ```
/// use poem::{Route, EndpointExt, get, handler, web::State, test::TestClient};
///
/// #[derive(Clone)]
/// struct AppState { value: i32 }
///
/// #[handler]
/// fn index(State(state): State<AppState>) -> String {
///     format!("{}", state.value)
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let app = Route::new()
///     .at("/", get(index))
///     .with_state(AppState { value: 42 });
///
/// let resp = TestClient::new(app).get("/").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("42").await;
/// # });
/// ```
pub trait Endpoint<S = ()>: Send + Sync {
    /// Represents the response of the endpoint.
    type Output: IntoResponse;

    /// Get the response to the request.
    fn call(&self, req: Request, state: &S) -> impl Future<Output = Result<Self::Output>> + Send;

    /// Get the response to the request and return a [`Response`].
    ///
    /// Unlike [`Endpoint::call`], when an error occurs, it will also convert
    /// the error into a response object.
    ///
    /// Note: This method is only available for `Endpoint<()>`.
    fn get_response(&self, req: Request) -> impl Future<Output = Response> + Send
    where
        S: Default + Send,
    {
        async move {
            self.call(req, &S::default())
                .await
                .map(IntoResponse::into_response)
                .unwrap_or_else(|err| err.into_response())
        }
    }
}

struct SyncFnEndpoint<T, F> {
    _mark: PhantomData<T>,
    f: F,
}

impl<F, T, R> Endpoint for SyncFnEndpoint<T, F>
where
    F: Fn(Request) -> R + Send + Sync,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    type Output = T;

    async fn call(&self, req: Request, _state: &()) -> Result<Self::Output> {
        (self.f)(req).into_result()
    }
}

struct AsyncFnEndpoint<T, F> {
    _mark: PhantomData<T>,
    f: F,
}

impl<F, Fut, T, R> Endpoint for AsyncFnEndpoint<T, F>
where
    F: Fn(Request) -> Fut + Sync + Send,
    Fut: Future<Output = R> + Send,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    type Output = T;

    async fn call(&self, req: Request, _state: &()) -> Result<Self::Output> {
        (self.f)(req).await.into_result()
    }
}

/// The enum `EitherEndpoint` with variants `Left` and `Right` is a general
/// purpose sum type with two cases.
pub enum EitherEndpoint<A, B> {
    /// A endpoint of type `A`
    A(A),
    /// A endpoint of type `B`
    B(B),
}

impl<S, A, B> Endpoint<S> for EitherEndpoint<A, B>
where
    S: Send + Sync,
    A: Endpoint<S>,
    B: Endpoint<S>,
{
    type Output = Response;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        match self {
            EitherEndpoint::A(a) => a.call(req, state).await.map(IntoResponse::into_response),
            EitherEndpoint::B(b) => b.call(req, state).await.map(IntoResponse::into_response),
        }
    }
}

/// Create an endpoint with a function.
///
/// The output can be any type that implements [`IntoResult`].
///
/// # Example
///
/// ```
/// use poem::{Endpoint, Request, endpoint::make_sync, http::Method, test::TestClient};
///
/// let ep = make_sync(|req| req.method().to_string());
/// let cli = TestClient::new(ep);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("GET").await;
/// # });
/// ```
pub fn make_sync<F, T, R>(f: F) -> impl Endpoint<Output = T>
where
    F: Fn(Request) -> R + Send + Sync,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    SyncFnEndpoint {
        _mark: PhantomData,
        f,
    }
}

/// Create an endpoint with a asyncness function.
///
/// The output can be any type that implements [`IntoResult`].
///
/// # Example
///
/// ```
/// use poem::{Endpoint, Request, endpoint::make, http::Method, test::TestClient};
///
/// let ep = make(|req| async move { req.method().to_string() });
/// let app = TestClient::new(ep);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = app.get("/").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("GET").await;
/// # });
/// ```
pub fn make<F, Fut, T, R>(f: F) -> impl Endpoint<Output = T>
where
    F: Fn(Request) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send,
    T: IntoResponse + Sync,
    R: IntoResult<T>,
{
    AsyncFnEndpoint {
        _mark: PhantomData,
        f,
    }
}

impl<S, T: Endpoint<S> + ?Sized> Endpoint<S> for &T
where
    S: Send + Sync,
{
    type Output = T::Output;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        T::call(self, req, state).await
    }
}

impl<S, T: Endpoint<S> + ?Sized> Endpoint<S> for Box<T>
where
    S: Send + Sync,
{
    type Output = T::Output;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        self.as_ref().call(req, state).await
    }
}

impl<S, T: Endpoint<S> + ?Sized> Endpoint<S> for Arc<T>
where
    S: Send + Sync,
{
    type Output = T::Output;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        self.as_ref().call(req, state).await
    }
}

/// A `endpoint` that can be dynamically dispatched.
pub trait DynEndpoint<S = ()>: Send + Sync {
    /// Represents the response of the endpoint.
    type Output: IntoResponse;

    /// Get the response to the request.
    fn call<'a>(&'a self, req: Request, state: &'a S) -> BoxFuture<'a, Result<Self::Output>>;
}

/// A [`Endpoint`] wrapper used to implement [`DynEndpoint`].
pub struct ToDynEndpoint<E>(pub E);

impl<S, E> DynEndpoint<S> for ToDynEndpoint<E>
where
    S: Send + Sync,
    E: Endpoint<S>,
{
    type Output = E::Output;

    #[inline]
    fn call<'a>(&'a self, req: Request, state: &'a S) -> BoxFuture<'a, Result<Self::Output>> {
        self.0.call(req, state).boxed()
    }
}

impl<S, T> Endpoint<S> for dyn DynEndpoint<S, Output = T> + '_
where
    S: Send + Sync,
    T: IntoResponse,
{
    type Output = T;

    #[inline]
    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        DynEndpoint::call(self, req, state).await
    }
}

/// An owned dynamically typed `Endpoint` for use in cases where you can't
/// statically type your result or need to add some indirection.
pub type BoxEndpoint<'a, S = (), T = Response> = Box<dyn DynEndpoint<S, Output = T> + 'a>;

/// Endpoint wrapper that provides state to an inner endpoint.
///
/// This is created by [`EndpointExt::with_state`].
///
/// `WithState<E, S>` wraps an `Endpoint<S>` (which requires state of type `S`)
/// and produces an `Endpoint<()>` (which requires no external state). The state
/// is captured and provided to the inner endpoint on each call.
pub struct WithState<E, S> {
    inner: E,
    state: S,
}

impl<E, S> WithState<E, S> {
    /// Create a new `WithState` endpoint.
    pub fn new(inner: E, state: S) -> Self {
        Self { inner, state }
    }
}

impl<E, S> Endpoint<()> for WithState<E, S>
where
    E: Endpoint<S>,
    S: Clone + Send + Sync + 'static,
{
    type Output = E::Output;

    async fn call(&self, mut req: Request, _state: &()) -> Result<Self::Output> {
        // Store the state in request extensions so State<T> can extract it
        // This is for backward compatibility with the State<T> extractor
        req.extensions_mut()
            .insert(crate::web::StateData(self.state.clone()));
        // Pass our captured state to the inner endpoint
        self.inner.call(req, &self.state).await
    }
}

/// Extension trait for [`Endpoint`].
pub trait EndpointExt<S = ()>: IntoEndpoint<S> {
    /// Wrap the endpoint in a Box.
    fn boxed<'a>(self) -> BoxEndpoint<'a, S, <Self::Endpoint as Endpoint<S>>::Output>
    where
        S: Send + Sync + 'static,
        Self: Sized + 'a,
    {
        Box::new(ToDynEndpoint(self.into_endpoint()))
    }

    /// Use middleware to transform this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Request, Route, get, handler, http::StatusCode, middleware::AddData,
    ///     test::TestClient, web::Data,
    /// };
    ///
    /// #[handler]
    /// async fn index(Data(data): Data<&i32>) -> String {
    ///     format!("{}", data)
    /// }
    ///
    /// let app = Route::new().at("/", get(index)).with(AddData::new(100i32));
    /// let cli = TestClient::new(app);
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli.get("/").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("100").await;
    /// # });
    /// ```
    fn with<T>(self, middleware: T) -> T::Output
    where
        T: Middleware<Self::Endpoint, S>,
        Self: Sized,
    {
        middleware.transform(self.into_endpoint())
    }

    /// if `enable` is `true` then use middleware to transform this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Request, Route, get, handler,
    ///     http::{StatusCode, Uri},
    ///     middleware::AddData,
    ///     test::TestClient,
    ///     web::Data,
    /// };
    ///
    /// #[handler]
    /// async fn index(data: Option<Data<&i32>>) -> String {
    ///     match data {
    ///         Some(data) => data.0.to_string(),
    ///         None => "none".to_string(),
    ///     }
    /// }
    ///
    /// let app = Route::new()
    ///     .at("/a", get(index).with_if(true, AddData::new(100i32)))
    ///     .at("/b", get(index).with_if(false, AddData::new(100i32)));
    /// let cli = TestClient::new(app);
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = cli.get("/a").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("100").await;
    ///
    /// let resp = cli.get("/b").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("none").await;
    /// # });
    /// ```
    fn with_if<T>(self, enable: bool, middleware: T) -> EitherEndpoint<Self, T::Output>
    where
        T: Middleware<Self::Endpoint, S>,
        Self: Sized,
    {
        if !enable {
            EitherEndpoint::A(self)
        } else {
            EitherEndpoint::B(middleware.transform(self.into_endpoint()))
        }
    }

    /// Attach a state data to the endpoint, similar to `with(AddData(T))`.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Request, handler, http::StatusCode, test::TestClient, web::Data,
    /// };
    ///
    /// #[handler]
    /// async fn index(data: Data<&i32>) -> String {
    ///     format!("{}", data.0)
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let ep = EndpointExt::<()>::data(index, 100i32);
    /// let resp = TestClient::new(ep).get("/").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("100").await;
    /// # });
    /// ```
    ///
    /// # Using with Trait Objects
    ///
    /// When using trait objects, you must explicitly coerce the value to the
    /// trait object type before passing it to this method. See the
    /// [`Data`](crate::web::Data) extractor documentation for a detailed
    /// explanation and examples.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use poem::{
    ///     Endpoint, EndpointExt, Request, handler, http::StatusCode, test::TestClient, web::Data,
    /// };
    ///
    /// trait Database: Send + Sync {
    ///     fn name(&self) -> &str;
    /// }
    ///
    /// struct PostgresDb;
    /// impl Database for PostgresDb {
    ///     fn name(&self) -> &str { "postgres" }
    /// }
    ///
    /// #[handler]
    /// async fn index(db: Data<&Arc<dyn Database>>) -> String {
    ///     db.name().to_string()
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// // Key: coerce to Arc<dyn Database> before calling .data()
    /// let db: Arc<dyn Database> = Arc::new(PostgresDb);
    /// let resp = TestClient::new(index.data(db)).get("/").send().await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("postgres").await;
    /// # });
    /// ```
    fn data<T>(self, data: T) -> AddDataEndpoint<Self::Endpoint, T>
    where
        T: Clone + Send + Sync + 'static,
        S: Send + Sync,
        Self: Sized,
    {
        self.with(AddData::new(data))
    }

    /// if `data` is `Some(T)` then attach the value to the endpoint.
    fn data_opt<T>(
        self,
        data: Option<T>,
    ) -> EitherEndpoint<AddDataEndpoint<Self::Endpoint, T>, Self>
    where
        T: Clone + Send + Sync + 'static,
        S: Send + Sync,
        Self: Sized,
    {
        match data {
            Some(data) => EitherEndpoint::A(AddData::new(data).transform(self.into_endpoint())),
            None => EitherEndpoint::B(self),
        }
    }

    /// Provide state for this endpoint, transforming `Endpoint<S>` into `Endpoint<()>`.
    ///
    /// This is the key method for compile-time state verification. An endpoint that
    /// requires state `S` cannot be served until `.with_state(state)` is called.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Request, handler, http::StatusCode, test::TestClient, web::State,
    /// };
    ///
    /// #[derive(Clone)]
    /// struct AppState {
    ///     value: i32,
    /// }
    ///
    /// #[handler]
    /// async fn index(State(state): State<AppState>) -> String {
    ///     format!("{}", state.value)
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = TestClient::new(index.with_state(AppState { value: 42 }))
    ///     .get("/")
    ///     .send()
    ///     .await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("42").await;
    /// # });
    /// ```
    ///
    /// # Extracting Substates
    ///
    /// You can extract substates by implementing [`FromRef`](crate::web::FromRef):
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Request, handler,
    ///     http::StatusCode,
    ///     test::TestClient,
    ///     web::{FromRef, State},
    /// };
    ///
    /// #[derive(Clone)]
    /// struct AppState {
    ///     db: DbState,
    /// }
    ///
    /// #[derive(Clone)]
    /// struct DbState {
    ///     connection: String,
    /// }
    ///
    /// impl FromRef<AppState> for DbState {
    ///     fn from_ref(state: &AppState) -> Self {
    ///         state.db.clone()
    ///     }
    /// }
    ///
    /// #[handler]
    /// async fn use_db(State(db): State<DbState>) -> String {
    ///     db.connection.clone()
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let state = AppState {
    ///     db: DbState {
    ///         connection: "postgres://localhost".to_string(),
    ///     },
    /// };
    /// // Use FromRef to derive the substate
    /// let db_state = DbState::from_ref(&state);
    /// let ep = EndpointExt::<DbState>::with_state(use_db, db_state);
    /// let resp = TestClient::new(ep)
    ///     .get("/")
    ///     .send()
    ///     .await;
    /// resp.assert_status_is_ok();
    /// resp.assert_text("postgres://localhost").await;
    /// # });
    /// ```
    fn with_state<T>(self, state: T) -> WithState<Self::Endpoint, T>
    where
        T: Clone + Send + Sync + 'static,
        Self: Sized,
    {
        WithState::new(self.into_endpoint(), state)
    }

    /// Maps the request of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Error, Request, Result, handler, http::StatusCode, test::TestClient,
    /// };
    ///
    /// #[handler]
    /// async fn index(data: String) -> String {
    ///     data
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let ep = EndpointExt::<()>::before(index, |mut req| async move {
    ///     req.set_body("abc");
    ///     Ok(req)
    /// });
    /// let mut resp = ep.call(Request::default(), &()).await.unwrap();
    /// assert_eq!(resp.take_body().into_string().await.unwrap(), "abc");
    /// # });
    /// ```
    fn before<F, Fut>(self, f: F) -> Before<Self, F>
    where
        F: Fn(Request) -> Fut + Send + Sync,
        Fut: Future<Output = Result<Request>> + Send,
        Self: Sized,
    {
        Before::new(self, f)
    }

    /// Maps the output of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{Endpoint, EndpointExt, Error, Request, Result, handler, http::StatusCode};
    ///
    /// #[handler]
    /// async fn index() -> &'static str {
    ///     "abc"
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let ep = EndpointExt::<()>::after(index, |res| async move {
    ///     match res {
    ///         Ok(resp) => Ok(resp.into_body().into_string().await.unwrap() + "def"),
    ///         Err(err) => Err(err),
    ///     }
    /// });
    /// let resp = ep.call(Request::default(), &()).await.unwrap();
    /// assert_eq!(resp, "abcdef");
    /// # });
    /// ```
    fn after<F, Fut, T>(self, f: F) -> After<Self::Endpoint, F>
    where
        F: Fn(Result<<Self::Endpoint as Endpoint<S>>::Output>) -> Fut + Send + Sync,
        Fut: Future<Output = Result<T>> + Send,
        T: IntoResponse,
        Self: Sized,
    {
        After::new(self.into_endpoint(), f)
    }

    /// Maps the request and response of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Error, Request, Response, Result, handler,
    ///     http::{HeaderMap, HeaderValue, StatusCode},
    /// };
    ///
    /// #[handler]
    /// async fn index(headers: &HeaderMap) -> String {
    ///     headers
    ///         .get("x-value")
    ///         .and_then(|value| value.to_str().ok())
    ///         .unwrap()
    ///         .to_string()
    ///         + ","
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp: String = index
    ///     .around(|ep, mut req, state: ()| async move {
    ///         req.headers_mut()
    ///             .insert("x-value", HeaderValue::from_static("hello"));
    ///         let mut resp: Response = ep.call(req, &state).await?;
    ///         Ok(resp.take_body().into_string().await.unwrap() + "world")
    ///     })
    ///     .call(Request::default(), &())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp, "hello,world");
    /// # });
    /// ```
    fn around<F, Fut, R>(self, f: F) -> Around<Self::Endpoint, F, S>
    where
        F: Fn(Arc<Self::Endpoint>, Request, S) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R>> + Send + 'static,
        R: IntoResponse,
        S: Clone + Send + Sync + 'static,
        Self: Sized,
    {
        Around::new(self.into_endpoint(), f)
    }

    /// Convert the output of this endpoint into a response.
    /// [`Response`](crate::Response).
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Error, Request, Response, Result, endpoint::make, http::StatusCode,
    /// };
    ///
    /// let ep1 = make(|_| async { "hello" }).map_to_response();
    /// let ep2 = make(|_| async { Err::<(), Error>(Error::from_status(StatusCode::BAD_REQUEST)) })
    ///     .map_to_response();
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = ep1.call(Request::default(), &()).await.unwrap();
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
    ///
    /// let err = ep2.call(Request::default(), &()).await.unwrap_err();
    /// assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    /// # });
    /// ```
    fn map_to_response(self) -> MapToResponse<Self::Endpoint, S>
    where
        Self: Sized,
    {
        MapToResponse::new(self.into_endpoint())
    }

    /// Convert the output of this endpoint into a response.
    /// [`Response`](crate::Response).
    ///
    /// NOTE: Unlike [`EndpointExt::map_to_response`], when an error occurs, it
    /// will also convert the error into a response object, so this endpoint
    /// will just returns `Ok(Response)`.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Error, Request, Response, Result, endpoint::make, http::StatusCode,
    /// };
    ///
    /// let ep1 = make(|_| async { "hello" }).to_response();
    /// let ep2 = make(|_| async { Err::<(), Error>(Error::from_status(StatusCode::BAD_REQUEST)) })
    ///     .to_response();
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = ep1.call(Request::default(), &()).await.unwrap();
    /// assert_eq!(resp.into_body().into_string().await.unwrap(), "hello");
    ///
    /// let resp = ep2.call(Request::default(), &()).await.unwrap();
    /// assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    /// # });
    /// ```
    fn to_response(self) -> ToResponse<Self::Endpoint, S>
    where
        Self: Sized,
    {
        ToResponse::new(self.into_endpoint())
    }

    /// Maps the response of this endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Error, Request, Response, Result, endpoint::make, http::StatusCode,
    /// };
    ///
    /// let ep = make(|_| async { "hello" }).map(|value| async move { format!("{}, world!", value) });
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let mut resp: String = ep.call(Request::default(), &()).await.unwrap();
    /// assert_eq!(resp, "hello, world!");
    /// # });
    /// ```
    fn map<F, Fut, R, R2>(self, f: F) -> Map<Self::Endpoint, F, S>
    where
        F: Fn(R) -> Fut + Send + Sync,
        Fut: Future<Output = R2> + Send,
        R: IntoResponse,
        R2: IntoResponse,
        Self: Sized,
        Self::Endpoint: Endpoint<S, Output = R> + Sized,
    {
        Map::new(self.into_endpoint(), f)
    }

    /// Calls `f` if the result is `Ok`, otherwise returns the `Err` value of
    /// self.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Error, Request, Response, Result, endpoint::make, http::StatusCode,
    /// };
    ///
    /// let ep1 = make(|_| async { "hello" })
    ///     .and_then(|value| async move { Ok(format!("{}, world!", value)) });
    /// let ep2 = make(|_| async { Err::<String, _>(Error::from_status(StatusCode::BAD_REQUEST)) })
    ///     .and_then(|value| async move { Ok(format!("{}, world!", value)) });
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp: String = ep1.call(Request::default(), &()).await.unwrap();
    /// assert_eq!(resp, "hello, world!");
    ///
    /// let err: Error = ep2.call(Request::default(), &()).await.unwrap_err();
    /// assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    /// # });
    /// ```
    fn and_then<F, Fut, R, R2>(self, f: F) -> AndThen<Self::Endpoint, F, S>
    where
        F: Fn(R) -> Fut + Send + Sync,
        Fut: Future<Output = Result<R2>> + Send,
        R: IntoResponse,
        R2: IntoResponse,
        Self: Sized,
        Self::Endpoint: Endpoint<S, Output = R> + Sized,
    {
        AndThen::new(self.into_endpoint(), f)
    }

    /// Catch all errors and convert it into a response.
    ///
    /// # Example
    ///
    /// ```
    /// use http::Uri;
    /// use poem::{
    ///     Endpoint, EndpointExt, Error, IntoResponse, Request, Response, Route, handler,
    ///     http::StatusCode, web::Json,
    /// };
    /// use serde::Serialize;
    ///
    /// #[handler]
    /// async fn index() {}
    ///
    /// let app = Route::new()
    ///     .at("/index", index)
    ///     .catch_all_error(custom_error);
    ///
    /// #[derive(Serialize)]
    /// struct ErrorResponse {
    ///     message: String,
    /// }
    ///
    /// async fn custom_error(err: Error) -> impl IntoResponse {
    ///     Json(ErrorResponse {
    ///         message: err.to_string(),
    ///     })
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/abc")).finish(), &())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp.status(), StatusCode::OK);
    /// assert_eq!(
    ///     resp.into_body().into_string().await.unwrap(),
    ///     "{\"message\":\"not found\"}"
    /// );
    /// # })
    /// ```
    fn catch_all_error<F, Fut, R>(self, f: F) -> CatchAllError<Self, F, R, S>
    where
        F: Fn(Error) -> Fut + Send + Sync,
        Fut: Future<Output = R> + Send,
        R: IntoResponse + Send,
        Self: Sized + Sync,
    {
        CatchAllError::new(self, f)
    }

    /// Catch the specified type of error and convert it into a response.
    ///
    /// # Example
    ///
    /// ```
    /// use http::Uri;
    /// use poem::{
    ///     Endpoint, EndpointExt, IntoResponse, Request, Response, Route, error::NotFoundError,
    ///     handler, http::StatusCode,
    /// };
    ///
    /// #[handler]
    /// async fn index() {}
    ///
    /// let app = Route::new().at("/index", index).catch_error(custom_404);
    ///
    /// async fn custom_404(_: NotFoundError) -> impl IntoResponse {
    ///     "custom not found".with_status(StatusCode::NOT_FOUND)
    /// }
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    ///
    /// let resp = app
    ///     .call(Request::builder().uri(Uri::from_static("/abc")).finish(), &())
    ///     .await
    ///     .unwrap();
    /// assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    /// assert_eq!(
    ///     resp.into_body().into_string().await.unwrap(),
    ///     "custom not found"
    /// );
    /// # })
    /// ```
    fn catch_error<F, Fut, R, ErrType>(self, f: F) -> CatchError<Self, F, R, ErrType, S>
    where
        F: Fn(ErrType) -> Fut + Send + Sync,
        Fut: Future<Output = R> + Send,
        R: IntoResponse + Send + Sync,
        ErrType: std::error::Error + Send + Sync + 'static,
        Self: Sized,
    {
        CatchError::new(self, f)
    }

    /// Does something with each error.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{EndpointExt, Route, handler};
    ///
    /// #[handler]
    /// fn index() {}
    ///
    /// let app = Route::<()>::new().at("/", index).inspect_all_err(|err| {
    ///     println!("error: {}", err);
    /// });
    /// ```
    fn inspect_all_err<F>(self, f: F) -> InspectAllError<Self, F, S>
    where
        F: Fn(&Error) + Send + Sync,
        Self: Sized,
    {
        InspectAllError::new(self, f)
    }

    /// Does something with each specified error type.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{EndpointExt, Route, error::NotFoundError, handler};
    ///
    /// #[handler]
    /// fn index() {}
    ///
    /// let app = Route::<()>::new()
    ///     .at("/", index)
    ///     .inspect_err(|err: &NotFoundError| {
    ///         println!("error: {}", err);
    ///     });
    /// ```
    fn inspect_err<F, ErrType>(self, f: F) -> InspectError<Self, F, ErrType, S>
    where
        F: Fn(&ErrType) + Send + Sync,
        ErrType: std::error::Error + Send + Sync + 'static,
        Self: Sized,
    {
        InspectError::new(self, f)
    }
}

impl<S, T: IntoEndpoint<S>> EndpointExt<S> for T {}

/// Represents a type that can convert into endpoint.
pub trait IntoEndpoint<S = ()> {
    /// Represents the endpoint type.
    type Endpoint: Endpoint<S>;

    /// Converts this object into endpoint.
    fn into_endpoint(self) -> Self::Endpoint;
}

impl<S, T: Endpoint<S>> IntoEndpoint<S> for T {
    type Endpoint = T;

    fn into_endpoint(self) -> Self::Endpoint {
        self
    }
}

#[cfg(test)]
mod test {
    use http::{HeaderValue, Uri};

    use crate::{
        Endpoint, EndpointExt, Error, IntoEndpoint, Request, Route,
        endpoint::{make, make_sync},
        get, handler,
        http::{Method, StatusCode},
        middleware::SetHeader,
        test::TestClient,
        web::Data,
    };

    #[tokio::test]
    async fn test_make() {
        let ep = make(|req| async move { format!("method={}", req.method()) }).map_to_response();
        let mut resp = ep
            .call(Request::builder().method(Method::DELETE).finish(), &())
            .await
            .unwrap();
        assert_eq!(
            resp.take_body().into_string().await.unwrap(),
            "method=DELETE"
        );
    }

    #[tokio::test]
    async fn test_before() {
        assert_eq!(
            make_sync(|req| req.method().to_string())
                .before(|mut req| async move {
                    req.set_method(Method::POST);
                    Ok(req)
                })
                .call(Request::default(), &())
                .await
                .unwrap(),
            "POST"
        );
    }

    #[tokio::test]
    async fn test_after() {
        assert_eq!(
            make_sync(|_| "abc")
                .after(|_| async { Ok::<_, Error>("def") })
                .call(Request::default(), &())
                .await
                .unwrap(),
            "def"
        );
    }

    #[tokio::test]
    async fn test_map_to_response() {
        assert_eq!(
            make_sync(|_| "abc")
                .map_to_response()
                .call(Request::default(), &())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "abc"
        );
    }

    #[tokio::test]
    async fn test_and_then() {
        assert_eq!(
            make_sync(|_| "abc")
                .and_then(|resp| async move { Ok(resp.to_string() + "def") })
                .call(Request::default(), &())
                .await
                .unwrap(),
            "abcdef"
        );

        let resp = make_sync(|_| Err::<String, _>(Error::from_status(StatusCode::BAD_REQUEST)))
            .and_then(|resp| async move { Ok(resp + "def") })
            .get_response(Request::default())
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_map() {
        assert_eq!(
            make_sync(|_| "abc")
                .map(|resp| async move { resp.to_string() + "def" })
                .call(Request::default(), &())
                .await
                .unwrap(),
            "abcdef"
        );

        let resp = make_sync(|_| Err::<String, _>(Error::from_status(StatusCode::BAD_REQUEST)))
            .map(|resp| async move { resp.to_string() + "def" })
            .get_response(Request::default())
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_around() {
        let ep = make(|req| async move { req.into_body().into_string().await.unwrap() + "b" });
        assert_eq!(
            ep.around(|ep, mut req, _state: ()| async move {
                req.set_body("a");
                let resp = ep.call(req, &()).await?;
                Ok(resp + "c")
            })
            .call(Request::default(), &())
            .await
            .unwrap(),
            "abc"
        );
    }

    #[tokio::test]
    async fn test_with_if() {
        let resp = make_sync(|_| ())
            .with_if(true, SetHeader::new().appending("a", 1))
            .call(Request::default(), &())
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get("a"),
            Some(&HeaderValue::from_static("1"))
        );

        let resp = make_sync(|_| ())
            .with_if(false, SetHeader::new().appending("a", 1))
            .call(Request::default(), &())
            .await
            .unwrap();
        assert_eq!(resp.headers().get("a"), None);
    }

    #[tokio::test]
    async fn test_into_endpoint() {
        struct MyEndpointFactory;

        impl IntoEndpoint for MyEndpointFactory {
            type Endpoint = Route;

            fn into_endpoint(self) -> Self::Endpoint {
                Route::new()
                    .at("/a", get(make_sync(|_| "a")))
                    .at("/b", get(make_sync(|_| "b")))
            }
        }

        let app = Route::new().nest("/api", MyEndpointFactory);

        assert_eq!(
            app.call(Request::builder().uri(Uri::from_static("/api/a")).finish(), &())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "a"
        );

        assert_eq!(
            app.call(Request::builder().uri(Uri::from_static("/api/b")).finish(), &())
                .await
                .unwrap()
                .take_body()
                .into_string()
                .await
                .unwrap(),
            "b"
        );
    }

    #[tokio::test]
    async fn test_data_opt() {
        #[handler(internal)]
        async fn index(data: Option<Data<&i32>>) -> String {
            match data.as_deref() {
                Some(value) => format!("{value}"),
                None => "none".to_string(),
            }
        }

        let ep = EndpointExt::<()>::data_opt(index, Some(100));
        let cli = TestClient::new(ep);
        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("100").await;

        let ep = EndpointExt::<()>::data_opt(index, None::<i32>);
        let cli = TestClient::new(ep);
        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_text("none").await;
    }
}
