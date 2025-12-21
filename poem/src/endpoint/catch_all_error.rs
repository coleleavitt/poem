use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, Error, IntoResponse, Request, Response, Result};

/// Endpoint for the [`catch_all_error`](super::EndpointExt::catch_all_error)
/// method.
pub struct CatchAllError<E, F, R, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<(R, S)>,
}

impl<E, F, R, S> CatchAllError<E, F, R, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> CatchAllError<E, F, R, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, Fut, R, S> Endpoint<S> for CatchAllError<E, F, R, S>
where
    E: Endpoint<S>,
    F: Fn(Error) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send,
    R: IntoResponse + Send + Sync,
    S: Send + Sync,
{
    type Output = Response;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        match self.inner.call(req, state).await {
            Ok(resp) => Ok(resp.into_response()),
            Err(err) => Ok((self.f)(err).await.into_response()),
        }
    }
}
