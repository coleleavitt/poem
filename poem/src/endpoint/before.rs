use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, Request, Result};

/// Endpoint for the [`before`](super::EndpointExt::before) method.
pub struct Before<E, F, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<S>,
}

impl<E, F, S> Before<E, F, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> Before<E, F, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, Fut, S> Endpoint<S> for Before<E, F, S>
where
    E: Endpoint<S>,
    F: Fn(Request) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Request>> + Send,
    S: Send + Sync,
{
    type Output = E::Output;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        self.inner.call((self.f)(req).await?, state).await
    }
}
