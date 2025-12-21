use std::marker::PhantomData;

use crate::{Endpoint, Error, Request, Result};

/// Endpoint for the [`inspect_all_err`](super::EndpointExt::inspect_all_err)
/// method.
pub struct InspectAllError<E, F, S = ()> {
    inner: E,
    f: F,
    _mark: PhantomData<S>,
}

impl<E, F, S> InspectAllError<E, F, S> {
    #[inline]
    pub(crate) fn new(inner: E, f: F) -> InspectAllError<E, F, S> {
        Self {
            inner,
            f,
            _mark: PhantomData,
        }
    }
}

impl<E, F, S> Endpoint<S> for InspectAllError<E, F, S>
where
    E: Endpoint<S>,
    F: Fn(&Error) + Send + Sync,
    S: Send + Sync,
{
    type Output = E::Output;

    async fn call(&self, req: Request, state: &S) -> Result<Self::Output> {
        match self.inner.call(req, state).await {
            Ok(resp) => Ok(resp),
            Err(err) => {
                (self.f)(&err);
                Err(err)
            }
        }
    }
}
