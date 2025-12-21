use std::marker::PhantomData;

use crate::{Endpoint, Request, Response, Result};

/// Endpoint for the [`to_response`](super::EndpointExt::to_response)
/// method.
pub struct ToResponse<E, S = ()> {
    inner: E,
    _mark: PhantomData<S>,
}

impl<E, S> ToResponse<E, S> {
    #[inline]
    pub(crate) fn new(inner: E) -> ToResponse<E, S> {
        Self {
            inner,
            _mark: PhantomData,
        }
    }
}

impl<E, S> Endpoint<S> for ToResponse<E, S>
where
    E: Endpoint<S>,
    S: Send + Sync,
{
    type Output = Response;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        use crate::IntoResponse;
        match self.inner.call(req, state).await {
            Ok(output) => Ok(output.into_response()),
            Err(err) => Ok(err.into_response()),
        }
    }
}
