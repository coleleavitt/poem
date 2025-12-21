use std::{future::Future, marker::PhantomData};

use crate::{Endpoint, IntoResponse, Request, Result};

/// Endpoint for the [`map_ok`](super::EndpointExt::map) method.
pub struct Map<E, F, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<S>,
}

impl<E, F, S> Map<E, F, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> Map<E, F, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, Fut, R, R2, S> Endpoint<S> for Map<E, F, S>
where
    E: Endpoint<S, Output = R>,
    F: Fn(R) -> Fut + Send + Sync,
    Fut: Future<Output = R2> + Send,
    R: IntoResponse,
    R2: IntoResponse,
    S: Send + Sync,
{
    type Output = R2;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        let resp = self.inner.call(req, state).await?;
        Ok((self.f)(resp).await)
    }
}
