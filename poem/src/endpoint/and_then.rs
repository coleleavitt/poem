use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, IntoResponse, Request, Result};

/// Endpoint for the [`and_then`](super::EndpointExt::and_then) method.
pub struct AndThen<E, F, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<S>,
}

impl<E, F, S> AndThen<E, F, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> AndThen<E, F, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, Fut, R, R2, S> Endpoint<S> for AndThen<E, F, S>
where
    E: Endpoint<S, Output = R>,
    F: Fn(R) -> Fut + Send + Sync,
    Fut: Future<Output = Result<R2>> + Send,
    R: IntoResponse,
    R2: IntoResponse,
    S: Send + Sync,
{
    type Output = R2;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        let resp = self.inner.call(req, state).await?;
        (self.f)(resp).await
    }
}
