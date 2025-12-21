use std::marker::PhantomData;

use crate::{Endpoint, IntoResponse, Request, Response, Result};

/// Endpoint for the [`map_to_response`](super::EndpointExt::map_to_response)
/// method.
pub struct MapToResponse<E, S = ()> {
    inner: E,
    _mark: PhantomData<S>,
}

impl<E, S> MapToResponse<E, S> {
    #[inline]
    pub(crate) fn new(inner: E) -> MapToResponse<E, S> {
        Self {
            inner,
            _mark: PhantomData,
        }
    }
}

impl<E, S> Endpoint<S> for MapToResponse<E, S>
where
    E: Endpoint<S>,
    S: Send + Sync,
{
    type Output = Response;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        self.inner
            .call(req, state)
            .await
            .map(IntoResponse::into_response)
    }
}
