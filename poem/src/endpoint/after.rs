use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, IntoResponse, Request, Result};

/// Endpoint for the [`after`](super::EndpointExt::after) method.
pub struct After<E, F, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<S>,
}

impl<E, F, S> After<E, F, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> After<E, F, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, Fut, T, S> Endpoint<S> for After<E, F, S>
where
    E: Endpoint<S>,
    F: Fn(Result<E::Output>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<T>> + Send,
    T: IntoResponse,
    S: Send + Sync,
{
    type Output = T;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        (self.f)(self.inner.call(req, state).await).await
    }
}
